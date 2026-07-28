//! This rule is by far the most significant rule in rtsim to date and governs
//! the behaviour of rtsim NPCs. It uses a novel combinator-based API to express
//! long-running NPC actions in a manner that's halfway between [async/coroutine programming](https://en.wikipedia.org/wiki/Coroutine) and traditional
//! [AI decision trees](https://en.wikipedia.org/wiki/Decision_tree).
//!
//! It may feel unintuitive when you first work with it, but trust us:
//! expressing your AI behaviour in this way brings radical advantages and will
//! simplify your code and make debugging exponentially easier.
//!
//! The fundamental abstraction is that of [`Action`]s. [`Action`]s, somewhat
//! like [`core::future::Future`], represent a long-running behaviour performed
//! by an NPC. See [`Action`] for a deeper explanation of actions and the
//! methods that can be used to combine them together.
//!
//! NPC actions act upon the NPC's [`crate::data::npc::Controller`]. This type
//! represent the immediate behavioural intentions of the NPC during simulation,
//! such as by specifying a location to walk to, an action to perform, speech to
//! say, or some persistent state to change (like the NPC's home site).
//!
//! After brain simulation has occurred, the resulting controller state is
//! passed to either rtsim's internal NPC simulation rule
//! ([`crate::rule::simulate_npcs`]) or, if the chunk the NPC is loaded, are
//! passed to the Veloren server's agent system which attempts to act in
//! accordance with it.

mod airship_ai;
#[cfg(feature = "airship_log")]
mod airship_logger;
/// bastion (B-AG2): archetype-keyed decision data — one shared brain,
/// many configs (see the converted gates in `villager`).
pub mod archetype;
pub mod dialogue;
pub mod movement;
pub mod quest;
pub mod util;

use std::{collections::VecDeque, hash::BuildHasherDefault, sync::Arc};

use crate::{
    RtState, Rule, RuleError,
    ai::{
        Action, NpcCtx, State, action_policy::ActionClassV1, choose, finish, just, now,
        predicate::{Chance, EveryRange, Predicate, every_range, timeout},
        seq, until,
    },
    data::{
        ReportKind, Sentiment, Sites,
        npc::{Brain, DialogueSession, Job, PathData, SimulationMode},
        quest::{Quest, QuestKind},
    },
    event::OnTick,
};
use common::{
    assets::AssetExt,
    astar::{Astar, PathResult},
    comp::{
        self, Content, bird_large,
        compass::{Direction, Distance},
        item::ItemDef,
    },
    map::{Marker, MarkerKind},
    match_some,
    path::Path,
    resources::Time,
    rtsim::{
        Actor, DialogueKind, ItemResource, NpcInput, NpcMsg, PersonalityTrait, Profession, QuestId,
        Response, Role, SiteId, TerrainResource,
    },
    spiral::Spiral2d,
    store::Id,
    terrain::{CoordinateConversions, TerrainChunkSize, sprite},
    threat_policy::{ThreatCandidateV1, ThreatClassV1, arbitrate},
    time::DayPeriod,
    util::{Dir, Dir2},
};
use core::ops::ControlFlow;
use fxhash::FxHasher64;
use itertools::{Either, Itertools};
use rand::{prelude::*, seq::IndexedRandom};
use rand_chacha::ChaChaRng;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use vek::*;
use world::{
    IndexRef, World,
    civ::{self, Track},
    site::{self, PlotKind, Site as WorldSite, SiteKind, Structure, TileKind, plot::tavern},
    util::NEIGHBORS,
};

use self::{
    movement::{
        follow_actor, goto, goto_2d, goto_2d_flying, goto_actor, travel_to_point, travel_to_site,
    },
    util::do_dialogue,
};

/// How many ticks should pass between running NPC AI.
/// Note that this only applies to simulated NPCs: loaded NPCs have their AI
/// code run every tick. This means that AI code should be broadly
/// DT-independent.
const SIMULATED_TICK_SKIP: u64 = 10;

pub struct NpcAi;

#[derive(Clone)]
struct DefaultState {
    socialize_timer: EveryRange,
    move_home_timer: Chance<EveryRange>,
}

impl Rule for NpcAi {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        // Keep track of the last `SIMULATED_TICK_SKIP` ticks, to know the deltatime
        // since the last tick we ran the npc.
        let mut last_ticks: VecDeque<_> = [1.0 / 30.0; SIMULATED_TICK_SKIP as usize]
            .into_iter()
            .collect();

        rtstate.bind::<Self, OnTick>(move |ctx| {
            last_ticks.push_front(ctx.event.dt);
            if last_ticks.len() >= SIMULATED_TICK_SKIP as usize {
                last_ticks.pop_back();
            }
            // Temporarily take the brains of NPCs out of their heads to appease the borrow
            // checker
            let mut npc_data = {
                let mut data = ctx.state.data_mut();
                data.npcs
                    .iter_mut()
                    // Don't run AI for dead NPCs
                    .filter(|(_, npc)| !npc.is_dead() && !matches!(npc.role, Role::Vehicle))
                    // Don't run AI for simulated NPCs every tick
                    .filter(|(_, npc)| matches!(npc.mode, SimulationMode::Loaded) || (npc.seed as u64 + ctx.event.tick).is_multiple_of(SIMULATED_TICK_SKIP))
                    .map(|(npc_id, npc)| {
                        let controller = std::mem::take(&mut npc.controller);
                        let inbox = std::mem::take(&mut npc.inbox);
                        let sentiments = std::mem::take(&mut npc.sentiments);
                        let known_reports = std::mem::take(&mut npc.known_reports);
                        let brain = npc.brain.take().unwrap_or_else(|| Brain {
                            action: Box::new(think().repeat().with_state(DefaultState {
                                socialize_timer: every_range(15.0..30.0),
                                move_home_timer: every_range(400.0..2000.0).chance(0.5),
                            })),
                        });
                        (npc_id, controller, inbox, sentiments, known_reports, brain, ctx.system_data.rtsim_gizmos.tracked.remove(npc_id))
                    })
                    .collect::<Vec<_>>()
            };

            // The sum of the last `SIMULATED_TICK_SKIP` tick deltatimes is the deltatime since
            // simulated npcs ran this tick had their ai ran.
            let simulated_dt = last_ticks.iter().sum::<f32>();

            // Do a little thinking
            {
                let data = &*ctx.state.data();

                npc_data
                    .par_iter_mut()
                    .for_each(|(npc_id, controller, inbox, sentiments, known_reports, brain, gizmos)| {
                        let npc = &data.npcs[*npc_id];

                        controller.reset(npc);

                        #[allow(unused)] // TODO: check if correct
                        brain.action.tick(&mut NpcCtx {
                            data,
                            world: ctx.world,
                            index: ctx.index,
                            time_of_day: ctx.event.time_of_day,
                            time: ctx.event.time,
                            npc,
                            npc_id: *npc_id,
                            controller,
                            inbox,
                            known_reports,
                            sentiments,
                            dt: if matches!(npc.mode, SimulationMode::Loaded) {
                                ctx.event.dt
                            } else {
                                simulated_dt
                            },
                            // DETRNG (B8 root fix): the per-NPC salt makes
                            // each stream independent of the par_iter order —
                            // deterministic under rayon by construction.
                            rng: crate::tick_rng(
                                ctx.index.seed,
                                ctx.event.tick,
                                npc.seed,
                            ),
                            dialogue_rng: crate::tick_rng(
                                ctx.index.seed,
                                ctx.event.tick,
                                npc.seed
                                    ^ crate::data::npc::Controller::DIALOGUE_ID_RNG_SALT,
                            ),
                            gizmos: gizmos.as_mut(),
                            system_data: &*ctx.system_data,
                            current_action_class: ActionClassV1::Social,
                        }, &mut ());

                        // If an input wasn't processed by the brain, we no longer have a use for it
                        inbox.clear();
                    });
            }

            // Reinsert NPC brains
            let mut data = ctx.state.data_mut();
            let mut to_update = Vec::with_capacity(npc_data.len());
            for (npc_id, controller, inbox, sentiments, known_reports, brain, gizmos) in npc_data {
                to_update.push(npc_id);
                data.npcs[npc_id].controller = controller;
                data.npcs[npc_id].brain = Some(brain);
                data.npcs[npc_id].inbox = inbox;
                data.npcs[npc_id].sentiments = sentiments;
                data.npcs[npc_id].known_reports = known_reports;

                if let Some(gizmos) = gizmos {
                    ctx.system_data.rtsim_gizmos.tracked.insert(npc_id, gizmos);
                }
            }
        });

        Ok(Self)
    }
}

fn idle<S: State>() -> impl Action<S> + Clone {
    just(|ctx, _| ctx.controller.do_idle()).debug(|| "idle")
}

fn talk_to<S: State>(tgt: Actor) -> impl Action<S> {
    now(move |ctx, _| {
        if ctx.sentiments.toward(tgt).is(Sentiment::ENEMY) {
            just(move |ctx, _| {
                ctx.controller
                    .say(tgt, Content::localized("npc-speech-reject_rival"))
            })
            .boxed()
        } else if matches!(tgt, Actor::Character(_)) {
            do_dialogue(tgt, move |session| dialogue::general(tgt, session)).boxed()
        } else {
            smalltalk_to(tgt).boxed()
        }
    })
}

fn tell_site_content(ctx: &NpcCtx, site: SiteId) -> Option<Content> {
    if let Some(world_site) = ctx.data.sites.get(site)
        && let Some(site_name) = util::site_name(ctx, site)
    {
        Some(
            Content::localized("npc-speech-tell_site")
                .with_arg("site", site_name)
                .with_arg(
                    "dir",
                    Direction::from_dir(world_site.wpos.as_() - ctx.npc.wpos.xy()).localize_npc(),
                )
                .with_arg(
                    "dist",
                    Distance::from_length(world_site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32)
                        .localize_npc(),
                ),
        )
    } else {
        None
    }
}

fn smalltalk_to<S: State>(tgt: Actor) -> impl Action<S> {
    now(move |ctx, _| {
        // t0.6-exempt: per-conversation-round draw, not a tick gate
        if matches!(tgt, Actor::Npc(_)) && ctx.rng.random_bool(0.2) {
            // Cut off the conversation sometimes to avoid infinite conversations (but only
            // if the target is an NPC!) TODO: Don't special case this, have
            // some sort of 'bored of conversation' system
            idle().boxed()
        } else {
            // Mention nearby sites
            // t0.6-exempt: one-shot content selection
            let comment = if ctx.rng.random_bool(0.3)
                && let Some(current_site) = ctx.npc.current_site
                && let Some(current_site) = ctx.data.sites.get(current_site)
                && let Some(mention_site) = current_site.nearby_sites_by_size.choose(&mut ctx.rng)
                && let Some(content) = tell_site_content(ctx, *mention_site)
            {
                content
            // Mention current site
            // t0.6-exempt: one-shot content/decision draw
            } else if ctx.rng.random_bool(0.3)
                && let Some(current_site) = ctx.npc.current_site
                && let Some(current_site_name) = util::site_name(ctx, current_site)
            {
                Content::localized("npc-speech-site").with_arg("site", current_site_name)

            // Mention nearby monsters
            // t0.6-exempt: one-shot content/decision draw
            } else if ctx.rng.random_bool(0.3)
                && let Some(monster) = ctx
                    .data
                    .npcs
                    .values()
                    .filter(|other| matches!(&other.role, Role::Monster))
                    .min_by_key(|other| other.wpos.xy().distance(ctx.npc.wpos.xy()) as i32)
            {
                Content::localized("npc-speech-tell_monster")
                    .with_arg("body", monster.body.localize_npc())
                    .with_arg(
                        "dir",
                        Direction::from_dir(monster.wpos.xy() - ctx.npc.wpos.xy()).localize_npc(),
                    )
                    .with_arg(
                        "dist",
                        Distance::from_length(monster.wpos.xy().distance(ctx.npc.wpos.xy()) as i32)
                            .localize_npc(),
                    )
            // Specific night dialog
            // t0.6-exempt: one-shot content selection
            } else if ctx.rng.random_bool(0.6) && DayPeriod::from(ctx.time_of_day.0).is_dark() {
                Content::localized("npc-speech-night")
            // t0.6-exempt: one-shot content/decision draw
            } else if ctx.rng.random_bool(0.3)
                && let Some(profession_comment) = match_some!(ctx.npc.profession(),
                    Some(Profession::Pirate(_)) => Content::localized("npc-speech-pirate"),
                )
            {
                profession_comment
            } else {
                ctx.npc.personality.get_generic_comment(&mut ctx.rng)
            };
            // TODO: Don't special-case players
            let wait = if matches!(tgt, Actor::Character(_)) {
                0.0
            } else {
                1.5
            };
            idle()
                .repeat()
                .stop_if(timeout(wait))
                .then(just(move |ctx, _| ctx.controller.say(tgt, comment.clone())))
                .boxed()
        }
    })
}

fn socialize() -> impl Action<EveryRange> {
    now(move |ctx, socialize: &mut EveryRange| {
        // Skip most socialising actions if we're not loaded
        if matches!(ctx.npc.mode, SimulationMode::Loaded)
            && socialize.should(ctx)
            && !ctx.npc.personality.is(PersonalityTrait::Introverted)
        {
            // Sometimes dance
            // t0.6-exempt: per-decision activity pick
            if ctx.rng.random_bool(0.15) {
                return just(|ctx, _| ctx.controller.do_dance(None))
                    .repeat()
                    .stop_if(timeout(6.0))
                    .debug(|| "dancing")
                    .map(|_, _| ())
                    .l()
                    .l();
            // Talk to nearby NPCs
            } else if let Some(other) = ctx
                .data
                .npcs
                .nearby(Some(ctx.npc_id), ctx.npc.wpos, 8.0)
                .choose(&mut ctx.rng)
            {
                return smalltalk_to(other)
                    // After talking, wait for a while
                    .then(idle().repeat().stop_if(timeout(4.0)))
                    .map(|_, _| ())
                    .r().l();
            }
        }
        idle().r()
    })
}

fn pirate(is_leader: bool) -> impl Action<DefaultState> {
    choose(move |ctx: &mut NpcCtx, _, consider| {
        if is_leader
            && let Some(home) = ctx.npc.home
            && ctx.npc.current_site == Some(home)
            && let Some(site) = ctx.data.sites.get(home)
            && let Some(faction) = ctx.npc.faction
            // Approx. once an hour.
            && ctx.chance(1.0 / 1200.0)
            && let Some(site_to_raid) = site
                .nearby_sites_by_size
                .iter()
                .filter(|site| {
                    ctx.data.sites.get(**site).is_some_and(|site| {
                        // Don't go further than 10km
                        site.wpos.as_::<f32>().distance_squared(ctx.npc.wpos.xy())
                            < 10000.0f32.powi(2)
                    })
                })
                .choose(&mut ctx.rng)
                .copied()
            && site
                .population
                .iter()
                .filter(|npc_id| {
                    ctx.data.npcs.get(**npc_id).is_some_and(|npc| {
                        !npc.is_dead()
                            && npc.current_site == Some(home)
                            && npc.faction == Some(faction)
                            && npc.hired().is_none()
                            && matches!(npc.role, Role::Civilised(Some(Profession::Pirate(false))))
                    })
                })
                .count()
                > 3
        {
            consider.important(
                now(move |ctx, _| {
                    if let Some(site) = ctx.data.sites.get(home)
                        && let Some(npc) = site
                            .population
                            .iter()
                            .filter(|npc_id| {
                                ctx.data.npcs.get(**npc_id).is_some_and(|npc| {
                                    !npc.is_dead()
                                        && npc.current_site == Some(home)
                                        && npc.faction == Some(faction)
                                        && npc.hired().is_none()
                                        && matches!(
                                            npc.role,
                                            Role::Civilised(Some(Profession::Pirate(false)))
                                        )
                                })
                            })
                            .choose(&mut ctx.rng)
                    {
                        let npc = *npc;
                        follow_actor(Actor::Npc(npc), 5.0)
                            .stop_if(move |ctx: &mut NpcCtx| {
                                let Some(follow_npc) = ctx.data.npcs.get(npc) else {
                                    return true;
                                };
                                ctx.npc.wpos.distance_squared(follow_npc.wpos) < 6.0f32.powi(2)
                            })
                            .then(just(move |ctx, _| ctx.controller.send_msg(npc, NpcMsg::RequestHire)))
                            .debug(|| "inviting raid participant")
                            .l()
                    } else {
                        idle().r()
                    }
                })
                .repeat()
                .stop_if(move |ctx: &mut NpcCtx| {
                    if let Some(site) = ctx.data.sites.get(home) {
                        let hired_count = site
                            .population
                            .iter()
                            .filter(|npc_id| {
                                ctx.data.npcs.get(**npc_id).is_some_and(|npc| {
                                    !npc.is_dead()
                                        && npc
                                            .hired()
                                            .is_some_and(|(a, _)| a == Actor::Npc(ctx.npc_id))
                                })
                            })
                            .count();

                        let unhired_count = site
                            .population
                            .iter()
                            .filter(|npc_id| {
                                ctx.data.npcs.get(**npc_id).is_some_and(|npc| {
                                    !npc.is_dead()
                                        && npc.current_site == Some(home)
                                        && npc.faction == Some(faction)
                                        && npc.hired().is_none()
                                        && matches!(
                                            npc.role,
                                            Role::Civilised(Some(Profession::Pirate(false)))
                                        )
                                })
                            })
                            .count();

                        if unhired_count == 0 {
                            return true;
                        }

                        let chance = match hired_count {
                            0..=3 => 0.0,
                            _ => (hired_count - 3) as f64 * 1.0 / 1200.0,
                        } / unhired_count as f64;

                        ctx.chance(chance)
                    } else {
                        true
                    }
                })
                .debug(|| "preparing for raid")
                .then(travel_to_site(site_to_raid, 0.8).debug(|| "travel to raid site"))
                .then(
                    // TODO: Replace this with raiding stuff
                    villager(site_to_raid)
                        .stop_if(timeout(ctx.rng.random_range(60.0..120.0)))
                        .debug(|| "raiding"),
                )
                .then(travel_to_site(home, 0.6).debug(|| "traveling home from raid"))
                // End hiring of hirlings
                .then(just(|ctx, _| {
                    if let Some(site) = ctx.npc.home
                        && let Some(site) = ctx.data.sites.get(site)
                    {
                        for &npc_id in site.population.iter() {
                            if let Some(npc) = ctx.data.npcs.get(npc_id)
                                && npc
                                    .hired()
                                    .is_some_and(|(actor, _)| actor == Actor::Npc(ctx.npc_id))
                            {
                                ctx.controller.send_msg(npc_id, NpcMsg::EndHire);
                            }
                        }
                    }
                }))
                .map(|_, _| ()),
            )
        } else if let Some((leader, _)) = ctx.npc.hired() {
            consider.important(
                follow_actor(leader, 5.0)
                    .stop_if(move |ctx: &mut NpcCtx| {
                        ctx.npc
                            .hired()
                            .is_none_or(move |(actor, _)| actor != leader)
                    })
                    .map(|_, _| ()),
            )
        } else if let Some(home) = ctx.npc.home {
            consider.casual(now(move |ctx, _| {
                let pos = ctx.data.sites.get(home).and_then(|site| {
                    let ws = ctx.index.sites.get(site.world_site?);
                    let plot = ws
                        .filter_plots(|plot| matches!(plot.kind(), PlotKind::PirateHideout(_)))
                        .choose(&mut ctx.rng)?;
                    let tile = plot.tiles().choose(&mut ctx.rng)?;
                    let wpos = ws.tile_center_wpos(tile);

                    Some(wpos.as_())
                });
                // Choose a plaza in the site we're visiting to walk to
                if let Some(new_pos) = pos {
                    // Walk to a point in the hideout...
                    Either::Left(travel_to_point(new_pos, 0.5)
                        .debug(|| "walk to pirate hideout"))
                } else {
                    // If there is no pirate hideout, unset the home.
                    ctx.controller.set_new_home(None);
                    Either::Right(finish())
                }
                    // ...then socialize for some time before moving on
                    .then(socialize()
                        .repeat()
                        .map_state(|state: &mut DefaultState| &mut state.socialize_timer)
                        .stop_if(timeout(ctx.rng.random_range(30.0..90.0)))
                        .debug(|| "wait at pirate hideout"))
                    .map(|_, _| ())
            }))
        } else {
            // Find new home
            consider.important(just(move |ctx, _| {
                if let Some((site, _)) =
                    ctx.data
                        .sites
                        .iter()
                        .filter(|(_, site)| {
                            site.world_site.is_some_and(|ws| {
                                ctx.index.sites.get(ws).any_plot(|plot| {
                                    matches!(plot.kind(), PlotKind::PirateHideout(_))
                                })
                            })
                        })
                        // DET-MIG-001 (v8 npc-migration, High): tie-break the
                        // nearest-site home search by site id, so an
                        // equal-distance choice is a pure function of the site
                        // set rather than iteration order.
                        .min_by_key(|(site_id, site)| {
                            (
                                site.wpos
                                    .as_::<i64>()
                                    .distance_squared(ctx.npc.wpos.xy().as_()),
                                *site_id,
                            )
                        })
                {
                    ctx.controller.set_new_home(site);
                }
            }))
        }
    })
}

fn adventure() -> impl Action<DefaultState> {
    choose(|ctx: &mut NpcCtx, _, consider| {
        // Choose a random site that's fairly close by
        if let Some(tgt_site) = ctx.data
            .sites
            .iter()
            .filter(|(site_id, site)| {
                site.world_site.is_some_and(|ws| ctx.index.sites.get(ws).any_plot(|p| p.is_workshop())) && (ctx.npc.current_site != Some(*site_id))
                    // t0.6-exempt: one-shot site-subset filter
                    && ctx.rng.random_bool(0.25)
            })
            // DET-MIG-001 (v8 npc-migration, High): tie-break the adventure
            // destination search by site id (canonical on equal distance).
            .min_by_key(|(site_id, site)| {
                (site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32, *site_id)
            })
            .map(|(site_id, _)| site_id)
        {
            let wait_time = if matches!(ctx.npc.profession(), Some(Profession::Merchant)) {
                60.0 * 15.0
            } else {
                60.0 * 3.0
            };
            let site_name = util::site_name(ctx, tgt_site).unwrap_or_default();
            // Travel to the site
            consider.important(just(move |ctx, _| ctx.controller.say(None, Content::localized("npc-speech-moving_on").with_arg("site", site_name.as_str())))
                .then(travel_to_site(tgt_site, 0.6))
                // Stop for a few minutes
                .then(villager(tgt_site).repeat().stop_if(timeout(wait_time)))
                .map(|_, _| ())
                .boxed())
        }
    })
    .debug(move || "adventure")
}

fn hired(tgt: Actor) -> impl Action<DefaultState> {
    follow_actor(tgt, 5.0)
        // Stop following if we're no longer hired
        .stop_if(move |ctx: &mut NpcCtx| ctx.npc.hired().is_none_or(|(a, _)| a != tgt))
        .debug(move|| format!("hired by {tgt:?}"))
        .interrupt_with(move |ctx, _| {
            // End hiring for various reasons
            if let Some((tgt, expires)) = ctx.npc.hired() {
                // Hiring period has expired (T0.11: world-clock compare)
                if ctx.time_of_day.0 > expires.0 {
                    ctx.controller.end_hiring();
                    // If the actor exists, tell them that the hiring is over
                    if util::actor_exists(ctx, tgt) {
                        return Some(goto_actor(tgt, 2.0)
                            .then(do_dialogue(tgt, |session| {
                                session.say_statement(Content::localized("npc-dialogue-hire_expired"))
                            }))
                            .boxed());
                    }
                }

                if ctx.sentiments.toward(tgt).is(Sentiment::RIVAL) {
                    ctx.controller.end_hiring();
                    // If the actor exists, tell them that the hiring is over
                    if util::actor_exists(ctx, tgt) {
                        return Some(goto_actor(tgt, 2.0)
                            .then(do_dialogue(tgt, |session| {
                                session.say_statement(Content::localized(
                                    "npc-dialogue-hire_cancelled_unhappy",
                                ))
                            }))
                            .boxed());
                    }
                }

                if let Some(visiting) = ctx.npc.current_site &&
                   let Some(visiting_site) = ctx.data.sites.get(visiting) &&
                   let Some(visiting_ws) = visiting_site.world_site &&
                   let Some(pos) = util::locate_actor(ctx, tgt) &&
                   let Some(chunk) = ctx.world.sim().get_wpos(pos.xy().as_()) &&
                   chunk.sites.contains(&visiting_ws) &&
                   let Some((pid, tavern)) = ctx.index.sites.get(visiting_ws).plots.iter().filter_map(|(pid, plot)| match_some!(plot.kind(), PlotKind::Tavern(t) => (pid, t))).choose(&mut ctx.npc.rng(14))
                   {
                    let tavern_name = tavern.name.clone();
                    return Some(just(move |ctx, _| {
                        ctx.controller.say(
                            tgt,
                            Content::localized("npc-dialogue-hire_arrive_tavern")
                                .with_arg("tavern", tavern_name.as_str())
                        )
                    })
                    .then(
                        go_to_tavern(visiting, pid).stop_if(move |ctx: &mut NpcCtx<'_, '_>| {
                            ctx.npc.hired().is_none_or(|(tgt, _)| {
                                util::locate_actor(ctx, tgt).is_none_or(|pos|
                                    ctx.world.sim()
                                        .get_wpos(pos.xy().as_())
                                        .is_none_or(|chunk|
                                            !chunk.sites.contains(&visiting_ws)
                                        )
                                )
                            })
                        })
                    )
                    .map(|_, _| ())
                    .boxed());
                }
            }

            None
        })
        .map(|_, _| ())
}

fn gather_ingredients<S: State>() -> impl Action<S> {
    just(|ctx, _| {
        ctx.controller.do_gather(
            &[
                TerrainResource::Fruit,
                TerrainResource::Mushroom,
                TerrainResource::Plant,
            ][..],
        )
    })
    .debug(|| "gather ingredients")
}

fn hunt_animals<S: State>() -> impl Action<S> {
    just(|ctx, _| ctx.controller.do_hunt_animals()).debug(|| "hunt_animals")
}

fn find_forest(ctx: &mut NpcCtx) -> Option<Vec2<f32>> {
    let chunk_pos = ctx.npc.wpos.xy().as_().wpos_to_cpos();
    Spiral2d::new()
        .skip(ctx.rng.random_range(1..=64))
        .take(24)
        .map(|rpos| chunk_pos + rpos)
        .find(|cpos| {
            ctx.world
                .sim()
                .get(*cpos)
                .is_some_and(|c| c.tree_density > 0.75 && c.surface_veg > 0.5)
        })
        .map(|chunk| TerrainChunkSize::center_wpos(chunk).as_())
}

fn find_farm(ctx: &mut NpcCtx, site: SiteId) -> Option<Vec2<f32>> {
    ctx.data.sites.get(site).and_then(|site| {
        let site = ctx.index.sites.get(site.world_site?);
        let farm = site
            .filter_plots(|p| matches!(p.kind(), PlotKind::FarmField(_)))
            .choose(&mut ctx.rng)?;

        Some(site.tile_center_wpos(farm.root_tile()).as_())
    })
}

fn choose_plaza(ctx: &mut NpcCtx, site: SiteId) -> Option<Vec2<f32>> {
    ctx.data.sites.get(site).and_then(|site| {
        let site = ctx.index.sites.get(site.world_site?);
        let plaza = &site.plots[site.plazas().choose(&mut ctx.rng)?];
        let tile = plaza
            .tiles()
            .choose(&mut ctx.rng)
            .unwrap_or_else(|| plaza.root_tile());
        Some(site.tile_center_wpos(tile).as_())
    })
}

const WALKING_SPEED: f32 = 0.35;

/// bastion (IDLE-HOME-LEASH, design §2): free-wander radius around the
/// colony anchor — inside it, idle wandering is fully unchanged (loose
/// orbit, not a huddle).
const IDLE_LEASH_RADIUS: f32 = 24.0;
/// bastion (IDLE-HOME-LEASH, design §2): the hard cap — an idle colonist
/// beyond this gets its NEXT wander target aimed back into the leash disc
/// (always an ordinary walk; never a teleport, never a forced state).
const IDLE_LEASH_MAX: f32 = 48.0;
/// bastion (IDLE-HOME-LEASH): wander-leg length band, blocks.
const IDLE_WANDER_LEG_MIN: f32 = 8.0;
const IDLE_WANDER_LEG_MAX: f32 = 16.0;

/// bastion (IDLE-HOME-LEASH): a uniform-ish random unit direction off the
/// NPC's own rng stream (the codebase's non-trig idiom; the slight corner
/// bias of the square sample is irrelevant at leash scale).
fn leash_rand_dir(rng: &mut impl rand::Rng) -> Vec2<f32> {
    Vec2::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0))
        .try_normalized()
        .unwrap_or_else(Vec2::unit_x)
}

/// bastion (IDLE-HOME-LEASH): a player-colony colonist's idle routine —
/// orbit the colony's home anchor instead of running [`villager`] at the
/// vanilla home SITE. A colonist's `npc.home` is the nearest worldgen TOWN
/// (set at colony spawn), so the vanilla brain walks idle colonists to that
/// town's houses/plazas — the AUTON-2 idle-drift that stranded colonists
/// 80+ blocks out, starving unreachable-from-food when the eat preempt
/// finally fired. This selector gates ONLY the next wander-leg TARGET:
/// movement remains the ordinary Goto/chaser pipeline (no new movement
/// writer), and jobs/hauling/needs/flee preemption drive the loaded agent
/// exactly as before regardless of this brain intent.
fn colonist_idle(anchor: Option<Vec3<f32>>) -> impl Action<DefaultState> {
    now(move |ctx, _| {
        let Some(anchor) = anchor else {
            // No stockpile and no Meeting zone yet (pre-bootstrap): leash
            // inactive — idle in place rather than town-walk (strictly
            // safer than the drift; the anchor appears with the first
            // painted stockpile and this arm re-evaluates).
            return idle()
                .repeat()
                .stop_if(timeout(5.0))
                .map(|_, _| ())
                .l();
        };
        let anchor = anchor.xy();
        let wpos = ctx.npc.wpos.xy();
        let dist = wpos.distance(anchor);
        // The next wander leg's target, leash-gated (design §2).
        let target = if dist > IDLE_LEASH_MAX {
            // Beyond the hard cap: re-aim INTO the leash disc around the
            // anchor (drift home over ordinary walk legs).
            let r = ctx.rng.random_range(0.0..IDLE_LEASH_RADIUS * 0.5);
            anchor + leash_rand_dir(&mut ctx.rng) * r
        } else if dist > IDLE_LEASH_RADIUS {
            // Soft band: two candidate legs, keep the anchor-ward one —
            // colonists drift back over a few legs, no beeline, no wall.
            let a = wpos
                + leash_rand_dir(&mut ctx.rng)
                    * ctx.rng.random_range(IDLE_WANDER_LEG_MIN..IDLE_WANDER_LEG_MAX);
            let b = wpos
                + leash_rand_dir(&mut ctx.rng)
                    * ctx.rng.random_range(IDLE_WANDER_LEG_MIN..IDLE_WANDER_LEG_MAX);
            if a.distance(anchor) <= b.distance(anchor) { a } else { b }
        } else {
            // Inside the leash: unchanged free wander (full local freedom —
            // the orbit look, not a huddle).
            wpos + leash_rand_dir(&mut ctx.rng)
                * ctx.rng.random_range(IDLE_WANDER_LEG_MIN..IDLE_WANDER_LEG_MAX)
        };
        // Selector invariant: NO emitted target lies outside the leash disc —
        // a leg from near the boundary is pulled radially inward, so any
        // position past IDLE_LEASH_MAX on tape is pathing wobble (small
        // slack), never the selector's doing.
        let target = {
            let off = target - anchor;
            let d = off.magnitude();
            if d > IDLE_LEASH_MAX {
                anchor + off / d * (IDLE_LEASH_RADIUS * 1.5)
            } else {
                target
            }
        };
        let wait = ctx.rng.random_range(4.0..12.0);
        travel_to_point(target, WALKING_SPEED)
            .debug(|| "colonist idle wander (home leash)")
            .then(
                socialize()
                    .repeat()
                    .map_state(|state: &mut DefaultState| &mut state.socialize_timer)
                    .stop_if(timeout(wait)),
            )
            .map(|_, _| ())
            .r()
    })
    .debug(|| "colonist idle (home leash)")
}

/// bastion (B-AG2): THE shared archetype gate — replaces the brain's
/// scattered `matches!(profession, X) && ctx.rng.random_bool(HARDCODED)`
/// pattern at converted sites with one lookup against the RON table
/// (`assets/common/rtsim/archetypes.ron`): same code path for every
/// archetype, the DATA decides. The rng rolls ONLY when the archetype
/// lists the activity — preserving each NPC's rng stream exactly as the
/// old short-circuit did (it rolled only on a profession match).
/// T0.6 (ledger #115): the gate is dt-INVARIANT — the RON weights keep
/// their historical meaning (per-evaluation probability at the 30 tps
/// cadence) and are mapped through the exact inverse
/// (`rate = 1 − (1 − w)^30`) into `NpcCtx::chance`'s per-second hazard, so
/// AI cadence changes no longer distort activity rates and the data file
/// needs no re-tuning.
fn archetype_gate(ctx: &mut NpcCtx, activity: &str) -> bool {
    archetype::archetype_key(ctx.npc.profession())
        .and_then(|key| archetype::archetype_chance(key, activity))
        .is_some_and(|w| {
            let rate = 1.0 - (1.0 - f64::from(w.clamp(0.0, 1.0))).powi(30);
            ctx.chance(rate)
        })
}

/// T3.27 (E3-W2, characterization-first, mandatory per the handoff):
/// which of [`villager`]'s three `consider.important(...)` branches
/// (mod.rs:923 migrate-home, :944 seek-house-at-night, :983 seek-
/// shelter-in-rain) wins under TODAY's `Consider::action` semantics --
/// first true condition wins by DECLARATION ORDER, not judged urgency
/// (`Consider::action`'s own doc explains why: with every real candidate
/// scored 0.0, the fixed hysteresis bonus makes whichever registers
/// first unbeatable by an equal-tier later candidate). This is a
/// hand-maintained mirror of the three conditions' ORDER and GUARD
/// EXEMPTIONS, not a live `NpcCtx` simulation of `villager()` itself
/// (which needs one) -- it pins the EMERGENT DECISION as a pure function
/// of the gating booleans, so a future migration's behavior diff is
/// explicit and reviewable rather than blind. Keep this in lockstep with
/// villager()'s actual branch order/guard-exemptions if either changes;
/// `migrate_eligible`'s own (unrelated, complex) eligibility computation
/// is an opaque input here, not re-derived.
///
/// Scope note, disclosed: only the three `important()`-tier branches are
/// characterized (the specific bug the handoff names). The `casual()`-
/// tier "fun activities" section below them (mod.rs:1080+) and any
/// further behavior are out of scope for this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "consumed by E3-W2's migration follow-up; live in the tree now so the characterization tests below can pin today's baseline first")]
enum VillagerImportantBranchV1 {
    MigrateHome,
    SeekHouseAtNight,
    SeekShelterInRain,
    None,
}

#[expect(dead_code, reason = "consumed by E3-W2's migration follow-up; live in the tree now so the characterization tests below can pin today's baseline first")]
fn villager_important_branch_today(
    migrate_eligible: bool,
    is_dark: bool,
    is_raining: bool,
    is_guard: bool,
) -> VillagerImportantBranchV1 {
    if migrate_eligible {
        return VillagerImportantBranchV1::MigrateHome;
    }
    if is_dark && !is_guard {
        return VillagerImportantBranchV1::SeekHouseAtNight;
    }
    if is_raining && !is_guard {
        return VillagerImportantBranchV1::SeekShelterInRain;
    }
    VillagerImportantBranchV1::None
}

#[cfg(test)]
mod villager_important_branch_characterization {
    use super::{VillagerImportantBranchV1::*, villager_important_branch_today as branch};

    /// The confirmed live bug the handoff names: dark AND raining
    /// simultaneously always picks "seek house at night" over "seek
    /// shelter from rain" -- an accident of source order (dark is
    /// checked at mod.rs:941, before rain at mod.rs:982), not a judgment
    /// that night shelter matters more than rain shelter. Pinned here so
    /// a future fix's diff is explicit against this documented baseline.
    #[test]
    fn dark_and_raining_picks_night_shelter_not_rain_shelter() {
        assert_eq!(branch(false, true, true, false), SeekHouseAtNight);
    }

    #[test]
    fn dark_only_picks_night_shelter() {
        assert_eq!(branch(false, true, false, false), SeekHouseAtNight);
    }

    #[test]
    fn rain_only_picks_rain_shelter() {
        assert_eq!(branch(false, false, true, false), SeekShelterInRain);
    }

    #[test]
    fn neither_dark_nor_raining_picks_nothing() {
        assert_eq!(branch(false, false, false, false), None);
    }

    /// Guards are exempted from both shelter branches (mod.rs:942,:982),
    /// even under conditions that would otherwise trigger them.
    #[test]
    fn guard_is_exempt_from_both_shelter_branches_even_dark_and_raining() {
        assert_eq!(branch(false, true, true, true), None);
    }

    /// Migrate-home is checked first (mod.rs:889) and, when eligible,
    /// always wins over dark/rain regardless of guard status -- migrate
    /// eligibility has no guard exemption in the real code.
    #[test]
    fn migrate_eligible_wins_over_dark_and_rain_regardless_of_guard() {
        assert_eq!(branch(true, true, true, false), MigrateHome);
        assert_eq!(branch(true, true, true, true), MigrateHome);
    }
}

fn villager(visiting_site: SiteId) -> impl Action<DefaultState> {
    choose(move |ctx, state: &mut DefaultState, consider| {
        // Consider moving home if the home site gets too full
        if state.move_home_timer.should(ctx)
            && let Some(home) = ctx.npc.home
            && Some(home) == ctx.npc.current_site
            && let Some(home_pop_ratio) = ctx.data.sites.get(home)
                .and_then(|site| Some((site, ctx.index.sites.get(site.world_site?))))
                .and_then(|(site, world_site)| { let houses = world_site.filter_plots(|p| p.is_house()).count(); if houses == 0 { return None } Some(site.population.len() as f32 / houses as f32) } )
                // Only consider moving if the population is more than 1.5x the number of homes
                .filter(|pop_ratio| *pop_ratio > 1.5)
            && let Some(new_home) = ctx.data
                .sites
                .iter()
                // Don't try to move to the site that's currently our home
                .filter(|(site_id, _)| Some(*site_id) != ctx.npc.home)
                // Only consider towns as potential homes
                .filter_map(|(site_id, site)| {
                    let world_site = site.world_site.map(|ws| ctx.index.sites.get(ws))?;
                    let house_count = world_site.filter_plots(|p| p.is_house()).count();

                    if house_count == 0 {
                        return None;
                    }
                    Some((site_id, site, house_count))
                })
                // Only select sites that are less densely populated than our own
                .filter(|(_, site, houses)| (site.population.len() as f32 / *houses as f32) < home_pop_ratio)
                // Find the closest of the candidate sites
                // DET-MIG-001 (v8 npc-migration, High): tie-break the migration
                // destination by site id (canonical on equal distance).
                .min_by_key(|(site_id, site, _)| {
                    (site.wpos.as_().distance(ctx.npc.wpos.xy()) as i32, *site_id)
                })
                .map(|(site_id, _, _)| site_id)
        {
            let site_name = util::site_name(ctx, new_home);
            consider.important(just(move |ctx, _| {
                if let Some(site_name) = &site_name {
                    ctx.controller.say(None, Content::localized("npc-speech-migrating").with_arg("site", site_name.as_str()))
                }
            })
                .then(travel_to_site(new_home, 0.5))
                .then(just(move |ctx, _| ctx.controller.set_new_home(new_home))));
        }

        let day_period = ctx.npc.get_day_period(ctx.time_of_day);
        let is_weekend = (ctx.time_of_day.day() as u64).is_multiple_of(6);
        let is_evening = day_period == DayPeriod::Evening;

        let is_free_time = is_weekend || is_evening;

        let is_raining = ctx.system_data.weather_grid.is_raining(ctx.npc.wpos.xy());

        // Go to a house if it's dark
        if day_period.is_dark()
            && !matches!(ctx.npc.profession(), Some(Profession::Guard))
        {
            consider.important(
                now(move |ctx, _| {
                    if let Some(house_wpos) = ctx.data
                        .sites
                        .get(visiting_site)
                        .and_then(|site| Some(ctx.index.sites.get(site.world_site?)))
                        .and_then(|site| {
                            // Find a house in the site we're visiting
                            let house = site
                                .plots()
                                .filter(|p| p.is_house())
                                .choose(&mut ctx.rng)?;
                            Some(site.tile_center_wpos(house.root_tile()).as_())
                        })
                    {
                        just(|ctx, _| {
                            ctx.controller
                                .say(None, Content::localized("npc-speech-night_time"))
                        })
                        .then(travel_to_point(house_wpos, 0.65))
                        .debug(|| "walk to house")
                        .then(socialize().repeat().map_state(|state: &mut DefaultState| &mut state.socialize_timer).debug(|| "wait in house"))
                        .stop_if(|ctx: &mut NpcCtx| ctx.npc.get_day_period(ctx.time_of_day).is_light())
                        .then(just(|ctx, _| {
                            ctx.controller
                                .say(None, Content::localized("npc-speech-day_time"))
                        }))
                        .map(|_, _| ())
                        .boxed()
                    } else {
                        finish().boxed()
                    }
                })
                .debug(|| "find somewhere to sleep"),
            );
        }

        // Go to a house if its raining
        if is_raining && !matches!(ctx.npc.profession(), Some(Profession::Guard)) {
            consider.important(
                now(move |ctx, _| {
                    if let Some(house_wpos) = ctx.data
                        .sites
                        .get(visiting_site)
                        .and_then(|site| Some(ctx.index.sites.get(site.world_site?)))
                        .and_then(|site| {
                            // Find a house in the site we're visiting
                            let house = site
                                .plots()
                                .filter(|p| p.is_house())
                                .choose(&mut ctx.rng)?;
                            Some(site.tile_center_wpos(house.root_tile()).as_())
                        })
                    {
                        just(|ctx, _| {
                                ctx.controller.say(None, Content::localized("npc-speech-seeking_shelter_rain"))
                        })
                        .then(travel_to_point(house_wpos, 0.65))
                        .debug(|| "walk to house (rain)")
                        .then(socialize().repeat().map_state(|state: &mut DefaultState| &mut state.socialize_timer).debug(|| "wait in house (rain)"))
                        .stop_if(|ctx: &mut NpcCtx| {
                                    let is_raining = ctx.system_data.weather_grid.is_raining(ctx.npc.wpos.xy());
                                    !is_raining
                    })
                        .then(just(|ctx, _| {
                                ctx.controller.say(None, Content::localized("npc-speech-rain_stopped"))
                        }))
                        .map(|_, _| ())
                        .boxed()
                        } else {
                        finish().boxed()
                    }
                })
                .debug(|| "find somewhere to wait (rain)"),
            );
        }

        // Go do something fun on evenings and holidays, or on random days.
        if
            // Ain't no rest for the wicked
            !matches!(ctx.npc.profession(), Some(Profession::Guard | Profession::Chef))
            // t0.6-exempt: per-day-plan decision draw
            && (matches!(day_period, DayPeriod::Evening) || is_free_time || ctx.rng.random_bool(0.05))
        {
            let mut fun_activities = Vec::new();

            if let Some(ws_id) = ctx.data.sites[visiting_site].world_site {
                let ws = ctx.index.sites.get(ws_id);
                if let Some(arena) = ws.plots().find_map(|p| match_some!(p.kind(), PlotKind::DesertCityArena(a) => a)) {
                    let wait_time = ctx.rng.random_range(100.0..300.0);
                    // We don't use Z coordinates for seats because they are complicated to calculate from the Ramp procedural generation
                    // and using goto_2d seems to work just fine. However it also means that NPC will never go seat on the stands
                    // on the first floor of the arena. This is a compromise that was made because in the current arena procedural generation
                    // there is also no pathways to the stands on the first floor for NPCs.
                    let arena_center = Vec3::new(arena.center.x, arena.center.y, arena.base).as_::<f32>();
                    let stand_dist = arena.stand_dist as f32;
                    let seat_var_width = ctx.rng.random_range(0..arena.stand_width) as f32;
                    let seat_var_length = ctx.rng.random_range(-arena.stand_length..arena.stand_length) as f32;
                    // Select a seat on one of the 4 arena stands
                    let seat = match ctx.rng.random_range(0..4) {
                        0 => Vec3::new(arena_center.x - stand_dist + seat_var_width, arena_center.y + seat_var_length, arena_center.z),
                        1 => Vec3::new(arena_center.x + stand_dist - seat_var_width, arena_center.y + seat_var_length, arena_center.z),
                        2 => Vec3::new(arena_center.x + seat_var_length, arena_center.y - stand_dist + seat_var_width, arena_center.z),
                        _ => Vec3::new(arena_center.x + seat_var_length, arena_center.y + stand_dist - seat_var_width, arena_center.z),
                    };
                    let look_dir = Dir::from_unnormalized(arena_center - seat);
                    // Walk to an arena seat, cheer, sit and dance
                    let action = just(move |ctx, _| ctx.controller.say(None, Content::localized("npc-speech-arena")))
                            .then(goto_2d(seat.xy(), 0.6, 1.0).debug(|| "go to arena"))
                            // Turn toward the centre of the arena and watch the action!
                            // t0.6-exempt: one-shot content/decision draw
                            .then(now(move |ctx, _| if ctx.rng.random_bool(0.3) {
                                just(move |ctx,_| ctx.controller.do_cheer(look_dir)).repeat().stop_if(timeout(5.0)).boxed()
                            // t0.6-exempt: one-shot content/decision draw
                            } else if ctx.rng.random_bool(0.15) {
                                just(move |ctx,_| ctx.controller.do_dance(look_dir)).repeat().stop_if(timeout(5.0)).boxed()
                            } else {
                                just(move |ctx,_| ctx.controller.do_sit(look_dir, None)).repeat().stop_if(timeout(15.0)).boxed()
                            })
                                .repeat()
                                .stop_if(timeout(wait_time)))
                            .map(|_, _| ())
                            .boxed();
                    fun_activities.push(action);
                }
                if let Some(tavern) = ws.plots.iter().filter_map(|(pid, p)| match_some!(p.kind(), PlotKind::Tavern(_) => pid)).choose(&mut ctx.rng) {
                    let wait_time = ctx.rng.random_range(100.0..300.0);
                    let action = go_to_tavern(visiting_site, tavern).stop_if(timeout(wait_time)).map(|_, _| ()).boxed();

                    fun_activities.push(action);
                }
            }


            if !fun_activities.is_empty() {
                let i = ctx.rng.random_range(0..fun_activities.len());
                consider.casual(fun_activities.swap_remove(i));
            }
        }

        // Villagers with roles should perform those roles.
        // bastion (B-AG2): the herbalist/hunter/guard gates below read the
        // archetype TABLE through one shared code path (weights + allowed
        // lists were the inline constants + matches!, moved verbatim to
        // data); farmer/merchant/chef convert in the §4 expansion pass.
        if archetype_gate(ctx, "gather_forest")
            && let Some(forest_wpos) = find_forest(ctx)
        {
            consider.casual(
                travel_to_point(forest_wpos, 0.5)
                    .debug(|| "walk to forest")
                    .then({
                        let wait_time = ctx.rng.random_range(10.0..30.0);
                        gather_ingredients().repeat().stop_if(timeout(wait_time))
                    })
                    .map(|_, _| ()),
            );
        }

        if matches!(ctx.npc.profession(), Some(Profession::Farmer))
            // t0.6-exempt: per-visit-plan decision draw
            && ctx.rng.random_bool(0.8)
            && let Some(farm_wpos) = find_farm(ctx, visiting_site)
        {
            consider.casual(
                travel_to_point(farm_wpos, 0.5)
                    .debug(|| "walk to farm")
                    .then({
                        let wait_time = ctx.rng.random_range(30.0..120.0);
                        gather_ingredients().repeat().stop_if(timeout(wait_time))
                    })
                    .map(|_, _| ()),
            );
        }

        if archetype_gate(ctx, "hunt_forest")
            && let Some(forest_wpos) = find_forest(ctx)
        {
            consider.casual(
                just(|ctx, _| {
                    ctx.controller
                        .say(None, Content::localized("npc-speech-start_hunting"))
                })
                .then(travel_to_point(forest_wpos, 0.75))
                .debug(|| "walk to forest")
                .then({
                    let wait_time = ctx.rng.random_range(30.0..60.0);
                    hunt_animals().repeat().stop_if(timeout(wait_time))
                })
                .map(|_, _| ()),
            );
        }

        if archetype_gate(ctx, "patrol_plaza")
            && let Some(plaza_wpos) = choose_plaza(ctx, visiting_site)
        {
            consider.casual(
                travel_to_point(plaza_wpos, 0.4)
                    .debug(|| "patrol")
                    .interrupt_with(move |ctx, _| {
                        // T0.6: polled every travel tick — per-second hazard
                        // (exact inverse of the old 0.0003/tick at 30 tps).
                        if ctx.chance(0.008960959398364388) {
                            Some(just(move |ctx, _| {
                                ctx.controller
                                    .say(None, Content::localized("npc-speech-guard_thought"))
                            }))
                        } else {
                            None
                        }
                    })
                    .map(|_, _| ()),
            );
        }

        // t0.6-exempt: per-visit-plan decision draw
        if matches!(ctx.npc.profession(), Some(Profession::Merchant)) && ctx.rng.random_bool(0.8) {
            consider.casual(
                just(|ctx, _| {
                    // Try to direct our speech at nearby actors, if there are any
                    // t0.6-exempt: one-shot speech-target choice
                    let (target, phrase) = if ctx.rng.random_bool(0.3) && let Some(other) = ctx.data
                        .npcs
                        .nearby(Some(ctx.npc_id), ctx.npc.wpos, 8.0)
                        .choose(&mut ctx.rng)
                    {
                        (Some(other), "npc-speech-merchant_sell_directed")
                    } else {
                        // Otherwise, resort to generic expressions
                        (None, "npc-speech-merchant_sell_undirected")
                    };

                    ctx.controller.say(target, Content::localized(phrase));
                })
                .then(idle().repeat().stop_if(timeout(8.0)))
                .repeat()
                .stop_if(timeout(60.0))
                .debug(|| "sell wares")
                .map(|_, _| ()),
            );
        }

        if matches!(ctx.npc.profession(), Some(Profession::Chef))
            // t0.6-exempt: per-visit-plan decision draw
            && ctx.rng.random_bool(0.8)
            && let Some(ws_id) = ctx.data.sites[visiting_site].world_site
            && let Some(tavern) = ctx.index.sites.get(ws_id).plots().filter_map(|p| match_some!(p.kind(), PlotKind::Tavern(a) => a)).choose(&mut ctx.rng)
            && let Some((bar_pos, room_center)) = tavern.rooms.values().flat_map(|room|
                room.details.iter().filter_map(|detail| match_some!(detail,
                    tavern::Detail::Bar { aabr } => {
                        let center = aabr.center();
                        (center.with_z(room.bounds.min.z), room.bounds.center().xy())
                    },
                ))
            ).choose(&mut ctx.rng) {

            let face_dir = Dir::from_unnormalized((room_center - bar_pos).as_::<f32>().with_z(0.0)).unwrap_or_else(|| Dir::random_2d(&mut ctx.rng));

            consider.casual(
                travel_to_point(tavern.door_wpos.xy().as_(), 0.5)
                    .then(goto(bar_pos.as_() + Vec2::new(0.5, 0.5), WALKING_SPEED, 2.0))
                    // TODO: Just dance there for now, in the future do other stuff.
                    .then(just(move |ctx, _| ctx.controller.do_dance(Some(face_dir))).repeat().stop_if(timeout(60.0)))
                    .debug(|| "cook food").map(|_, _| ())
            );
        }

        // If nothing else needs doing, walk between plazas and socialize
        consider.casual(now(move |ctx, _| {
            // Choose a plaza in the site we're visiting to walk to
            if let Some(plaza_wpos) = choose_plaza(ctx, visiting_site) {
                // Walk to the plaza...
                Either::Left(travel_to_point(plaza_wpos, 0.5)
                    .debug(|| "walk to plaza"))
            } else {
                // No plazas? :(
                Either::Right(finish())
            }
                // ...then socialize for some time before moving on
                .then(socialize()
                    .repeat()
                    .map_state(|state: &mut DefaultState| &mut state.socialize_timer)
                    .stop_if(timeout(ctx.rng.random_range(30.0..90.0)))
                    .debug(|| "wait at plaza"))
                .map(|_, _| ())
        }));
    })
    .debug(move || format!("villager at site {:?}", visiting_site))
}

fn go_to_tavern(site_id: SiteId, tavern_plot: Id<site::Plot>) -> impl Action<DefaultState> {
    now(move |ctx, _| {
        if let Some(site) = ctx.data.sites.get(site_id)
            && let Some(ws) = site.world_site
            && let PlotKind::Tavern(tavern) = ctx.index.sites.get(ws).plots.get(tavern_plot).kind()
        {
            let tavern_name = tavern.name.clone();
            let (stage_aabr, stage_z) = tavern
                .rooms
                .values()
                .flat_map(|room| {
                    room.details.iter().filter_map(|detail| {
                        match_some!(detail, tavern::Detail::Stage { aabr } => (*aabr, room.bounds.min.z + 1))
                    })
                })
                .choose(&mut ctx.rng)
                .unwrap_or((tavern.bounds, tavern.door_wpos.z));

            let bar_pos = tavern
                .rooms
                .values()
                .flat_map(|room| {
                    room.details.iter().filter_map(|detail| match_some!(detail,
                        tavern::Detail::Bar { aabr } => {
                            let side = Dir2::from_vec2(
                                room.bounds.center().xy() - aabr.center(),
                            );
                            let pos = side.select_aabr_with(*aabr, aabr.center()) + side.to_vec2();

                            pos.with_z(room.bounds.min.z)
                        },
                    ))
                })
                .choose(&mut ctx.rng)
                .unwrap_or(stage_aabr.center().with_z(stage_z));

            // Pick a chair that is theirs for the stay
            let chair_pos = tavern.rooms.values().flat_map(|room| {
            let z = room.bounds.min.z;
            room.details.iter().filter_map(move |detail| match_some!(detail,
                tavern::Detail::Table { pos, chairs } => chairs.into_iter().map(move |dir| pos.with_z(z) + dir.to_vec2())
            ))
            .flatten()
        }
        ).choose(&mut ctx.rng)
        // This path is possible, but highly unlikely.
        .unwrap_or(bar_pos);

            let stage_aabr = stage_aabr.as_::<f32>();
            let stage_z = stage_z as f32;

            travel_to_point(tavern.door_wpos.xy().as_() + 0.5, 0.8).then(now(move |ctx, (last_action, _)| {
                let action = [0, 1, 2].into_iter().filter(|i| *last_action != Some(*i)).choose(&mut ctx.rng).expect("We have at least 2 elements");
                let socialize_repeat = || socialize().map_state(|(_, timer)| timer).repeat();
                match action {
                    // Go and dance on a stage.
                    0 => now(move |ctx, (last_action, _)| {
                            *last_action = Some(action);
                            goto(stage_aabr.min.map2(stage_aabr.max, |a, b| ctx.rng.random_range(a..b)).with_z(stage_z), WALKING_SPEED, 1.0)
                        })
                        .then(just(move |ctx,_| ctx.controller.do_dance(None)).repeat().stop_if(timeout(ctx.rng.random_range(20.0..30.0))))
                        .map(|_, _| ())
                        .debug(|| "Dancing on the stage")
                        .boxed(),
                    // Go and sit at a table.
                    1 => now(move |ctx, (last_action, _)| {
                            *last_action = Some(action);
                            goto(chair_pos.as_() + 0.5, WALKING_SPEED, 1.0)
                                .then(just(move |ctx, _| ctx.controller.do_sit(None, Some(chair_pos)))
                                    // .then(socialize().map_state(|(_, timer)| timer))
                                    .repeat().stop_if(timeout(ctx.rng.random_range(30.0..60.0)))
                                )
                                .map(|_, _| ())
                        })
                        .debug(move || format!("Sitting in a chair at {} {} {}", chair_pos.x, chair_pos.y, chair_pos.z))
                        .boxed(),
                    // Go to the bar.
                    _ => now(move |ctx, (last_action, _)| {
                            *last_action = Some(action);
                            goto(bar_pos.as_() + 0.5, WALKING_SPEED, 1.0).then(socialize_repeat().stop_if(timeout(ctx.rng.random_range(10.0..25.0)))).map(|_, _| ())
                        })
                        .debug(|| "At the bar")
                        .boxed(),
                }
            })
            .with_state((None::<u32>, every_range(5.0..10.0)))
            .repeat())
            .map(|_, _| ())
            .debug(move || format!("At the tavern '{}'", tavern_name))
            .l()
        } else {
            just(|_, _| {}).r()
        }
    })
}

fn pilot<S: State>(ship: common::comp::ship::Body) -> impl Action<S> {
    // Travel between different towns in a straight line
    now(move |ctx, _| {
        let station_wpos = ctx
            .data
            .sites
            .iter()
            .filter(|(id, _)| Some(*id) != ctx.npc.current_site)
            .filter_map(|(_, site)| Some(ctx.index.sites.get(site.world_site?)))
            .flat_map(|site| {
                site.filter_plots(|p| p.airship_dock_info().is_some())
                    .map(|plot| site.tile_center_wpos(plot.root_tile()))
            })
            .choose(&mut ctx.rng);
        if let Some(station_wpos) = station_wpos {
            Either::Right(
                goto_2d_flying(
                    station_wpos.as_(),
                    1.0,
                    50.0,
                    150.0,
                    110.0,
                    ship.flying_height(),
                )
                .then(goto_2d_flying(
                    station_wpos.as_(),
                    1.0,
                    10.0,
                    32.0,
                    16.0,
                    30.0,
                )),
            )
        } else {
            Either::Left(finish())
        }
    })
    .repeat()
    .map(|_, _| ())
}

fn captain<S: State>() -> impl Action<S> {
    // For now just randomly travel the sea
    now(|ctx, _| {
        let chunk = ctx.npc.wpos.xy().as_().wpos_to_cpos();
        if let Some(chunk) = NEIGHBORS
            .into_iter()
            .map(|neighbor| chunk + neighbor)
            .filter(|neighbor| {
                ctx.world
                    .sim()
                    .get(*neighbor)
                    .is_some_and(|c| c.river.river_kind.is_some())
            })
            .choose(&mut ctx.rng)
        {
            let wpos = TerrainChunkSize::center_wpos(chunk);
            let wpos = wpos.as_().with_z(
                ctx.world
                    .sim()
                    .get_interpolated(wpos, |chunk| chunk.water_alt)
                    .unwrap_or(0.0),
            );
            goto(wpos, 0.7, 5.0).boxed()
        } else {
            idle().boxed()
        }
    })
    .repeat()
    .map(|_, _| ())
}

fn check_inbox<S: State>(ctx: &mut NpcCtx) -> Option<impl Action<S> + use<S>> {
    let mut action = None;
    ctx.inbox.retain(|input| {
        match input {
            NpcInput::Report(report_id) if !ctx.known_reports.contains(report_id) => {
                let Some(report) = ctx.data.reports.get(*report_id) else {
                    return false;
                };

                const REPORT_RESPONSE_TIME: f64 = 60.0 * 5.0;

                match report.kind {
                    ReportKind::Death { killer, actor, .. }
                        if matches!(&ctx.npc.role, Role::Civilised(_)) =>
                    {
                        // TODO: Don't report self
                        let phrase = if let Some(killer) = killer {
                            // TODO: For now, we don't make sentiment changes if the killer was an
                            // NPC in some cases because some NPCs can't hurt one-another.
                            // This should be changed in the future.
                            let can_damage_killer = if let Actor::Npc(killer) = killer {
                                ctx.data.npcs.get(killer).is_some_and(|killer| {
                                    match (&ctx.npc.role, &killer.role) {
                                        (Role::Vehicle, _) | (_, Role::Vehicle) => false,
                                        (Role::Civilised(prof_a), Role::Civilised(prof_b)) => {
                                            match (prof_a, prof_b) {
                                                (
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                ) => false,
                                                (
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                    _,
                                                )
                                                | (
                                                    _,
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                ) => true,

                                                _ => false,
                                            }
                                        },
                                        (Role::Civilised(_), _) => true,
                                        (Role::Wild, Role::Wild) => false,
                                        (Role::Wild, _) => true,
                                        (Role::Monster, Role::Monster) => false,
                                        (Role::Monster, _) => true,
                                    }
                                })
                            } else {
                                true
                            };

                            // TODO: Roles themselves are kind of a hack, and so is this. This is
                            // mostly a fix for npcs getting angry if you kill for example an ogre.
                            let is_victim_inherent_enemy = if let Actor::Npc(victim) = actor {
                                ctx.data.npcs.get(victim).is_some_and(|victim| {
                                    match (&ctx.npc.role, &victim.role) {
                                        (Role::Civilised(prof), Role::Civilised(victim_prof)) => {
                                            match (prof, victim_prof) {
                                                (
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                ) => false,
                                                (
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                    _,
                                                )
                                                | (
                                                    _,
                                                    Some(
                                                        Profession::Pirate(_) | Profession::Cultist,
                                                    ),
                                                ) => true,

                                                _ => false,
                                            }
                                        },

                                        (Role::Civilised(_), Role::Monster) => true,
                                        _ => false,
                                    }
                                })
                            } else {
                                false
                            };

                            let is_victim_enemy = is_victim_inherent_enemy
                                || ctx.sentiments.toward(actor).is(Sentiment::ENEMY);

                            if can_damage_killer {
                                // TODO: Don't hard-code sentiment change
                                let change = if is_victim_enemy {
                                    // Like the killer if we have negative sentiment towards the
                                    // killed.
                                    0.25
                                } else {
                                    -0.75
                                };
                                ctx.sentiments
                                    .toward_mut(killer)
                                    .change_by(change, Sentiment::VILLAIN);
                            }

                            // This is a murder of a player. Feel bad for the player and stop
                            // attacking them.
                            if let Actor::Character(_) = actor {
                                ctx.sentiments.toward_mut(actor).limit_below(0.1);
                                ctx.sentiments
                                    .toward_mut(actor)
                                    .change_by(Sentiment::NEGATIVE, Sentiment::NEGATIVE);

                                "npc-speech-witness_enemy_murder"
                            } else if is_victim_enemy {
                                "npc-speech-witness_enemy_murder"
                            } else {
                                "npc-speech-witness_murder"
                            }
                        } else {
                            "npc-speech-witness_death"
                        };
                        ctx.known_reports.insert(*report_id);

                        if ctx.time_of_day.0 - report.at_tod.0 < REPORT_RESPONSE_TIME {
                            action = Some(
                                just(move |ctx, _| {
                                    ctx.controller.say(killer, Content::localized(phrase))
                                })
                                .boxed(),
                            );
                        }
                        false
                    },
                    ReportKind::Theft {
                        thief,
                        site,
                        sprite,
                    } => {
                        // Check if this happened at home, where we know what belongs to who
                        if let Some(site) = site
                            && ctx.npc.home == Some(site)
                        {
                            // TODO: Don't hardcode sentiment change.
                            ctx.sentiments
                                .toward_mut(thief)
                                .change_by(-0.2, Sentiment::ENEMY);
                            ctx.known_reports.insert(*report_id);

                            let phrase = if matches!(ctx.npc.profession(), Some(Profession::Farmer))
                                && matches!(sprite.category(), sprite::Category::Plant)
                            {
                                "npc-speech-witness_theft_owned"
                            } else {
                                "npc-speech-witness_theft"
                            };

                            if ctx.time_of_day.0 - report.at_tod.0 < REPORT_RESPONSE_TIME {
                                action = Some(
                                    just(move |ctx, _| {
                                        ctx.controller.say(thief, Content::localized(phrase))
                                    })
                                    .boxed(),
                                );
                            }
                        }
                        false
                    },
                    // We don't care about deaths of non-civilians
                    ReportKind::Death { .. } => false,
                }
            },
            NpcInput::Report(_) => false, // Reports we already know of are ignored
            NpcInput::Interaction(by) => {
                action = Some(talk_to(*by).boxed());
                false
            },
            // Dialogue inputs get retained because they're handled by specific conversation actions
            // later
            NpcInput::Dialogue(_, _) => true,
            NpcInput::Msg {
                from,
                msg: NpcMsg::RequestHire,
            } => {
                let from = *from;
                action = Some(
                    idle()
                        .repeat()
                        .stop_if(timeout(2.0))
                        .then(just(move |ctx, _| {
                            ctx.controller
                                .say(from, Content::localized("npc-response-accept_hire"));
                            ctx.controller
                                .set_newly_hired(
                                    from,
                                    common::resources::TimeOfDay(f64::INFINITY),
                                );
                        }))
                        .boxed(),
                );
                false
            },
            NpcInput::Msg {
                from,
                msg: NpcMsg::EndHire,
            } => {
                // End hiring at the request of the hirer
                if matches!(ctx.controller.job, Some(Job::Hired(hirer, _)) if hirer == *from) {
                    ctx.controller.end_hiring();
                }
                false
            },
        }
    });

    action
}

/// `T3.35+T3.39` (E3-WT, Fable-ruled 2026-07-27): wires the shared
/// `threat_policy` in place of the old canonical-`Actor`-order tiebreak
/// (DET-AIT-004's fix, now superseded rather than removed — see below).
/// Disclosed collapse from 3 classes to 1: `NpcCtx` here only carries a
/// static `Sentiment::ENEMY` relationship and position, no per-actor
/// engagement/recency tracking, so `AttackingMe`/`AttackingAlly` can't be
/// honestly discriminated at this call site (that data lives only in
/// server-agent's `health.last_change` — see
/// `is_more_dangerous_than_target`). Every candidate here is therefore the
/// fixed class `HostileNearby`; `capability_vs_me`/`recency` are 0.0 (no
/// signal exists), so ranking reduces to real proximity (previously this
/// site had none at all) with `Actor`'s own total order as the exact-tie
/// tiebreak — DET-AIT-004's canonical-order fix is preserved as
/// `threat_policy::compare`'s tiebreak term, not lost.
fn check_for_enemies<S: State>(ctx: &mut NpcCtx) -> Option<impl Action<S> + use<S>> {
    // TODO: Instead of checking all nearby actors every tick, it would be more
    // effective to have the actor grid generate a per-tick diff so that we only
    // need to check new actors in the local area. Be careful though:
    // implementing this means accounting for changes in sentiment (that could
    // suddenly make a nearby actor an enemy) as well as variable NPC tick
    // rates!
    let wpos = ctx.npc.wpos;
    let candidates = ctx
        .data
        .npcs
        .nearby_with_pos(Some(ctx.npc_id), wpos, 24.0)
        .filter(|(actor, _)| ctx.sentiments.toward(*actor).is(Sentiment::ENEMY));

    pick_hostile_threat(wpos, candidates).map(|enemy| just(move |ctx, _| ctx.controller.attack(enemy)))
}

/// Pure core of [`check_for_enemies`]'s wiring, extracted so it's
/// unit-testable without a live `NpcCtx` (same discipline as
/// [`reaction_precedence`] above). All candidates are `HostileNearby`
/// (this call site's disclosed collapse, see `check_for_enemies`'s own
/// doc); ranking is real proximity with `Actor`'s total order as the
/// exact-tie tiebreak.
fn pick_hostile_threat(
    wpos: Vec3<f32>,
    candidates: impl IntoIterator<Item = (Actor, Vec3<f32>)>,
) -> Option<Actor> {
    let candidates = candidates
        .into_iter()
        .map(|(actor, pos)| ThreatCandidateV1 {
            class: ThreatClassV1::HostileNearby,
            distance: pos.distance(wpos),
            capability_vs_me: 0.0,
            recency: 0.0,
            tiebreak: actor,
        })
        .collect::<Vec<_>>();

    arbitrate(&candidates).map(|i| candidates[i].tiebreak)
}

fn threat_reaction<S: State>(ctx: &mut NpcCtx) -> Option<Box<dyn Action<S>>> {
    check_for_enemies(ctx).map(Action::boxed)
}
fn deadline_reaction<S: State>(ctx: &mut NpcCtx) -> Option<Box<dyn Action<S>>> {
    quest::check_for_timeouts(ctx).map(Action::boxed)
}
fn inbox_reaction<S: State>(ctx: &mut NpcCtx) -> Option<Box<dyn Action<S>>> {
    check_inbox::<S>(ctx).map(Action::boxed)
}

/// T3.34 (E3, Fable-ruled 2026-07-27): the reaction-precedence combinator
/// — threat > deadline > inbox. Was inbox > threat > deadline (an
/// accident of `.or_else()` declaration order, never a designed policy).
/// Generic over `C`/`T` (not hardcoded to `NpcCtx`/`Action`) so
/// [`reaction_precedence_tests`] exercises this EXACT function — the one
/// `react_to_events` calls — without needing a live `NpcCtx`. Each
/// candidate is still evaluated lazily, one `fn` call at a time (a plain
/// reborrow of `ctx` per call, never three simultaneous borrows), so a
/// lower-precedence check's side effects (inbox drainage, quest-timeout
/// resolution) only fire when that check is actually reached — same
/// conditionality as before this row, only the order changed.
fn reaction_precedence<C, T>(
    ctx: &mut C,
    threat: fn(&mut C) -> Option<T>,
    deadline: fn(&mut C) -> Option<T>,
    inbox: fn(&mut C) -> Option<T>,
) -> Option<T> {
    threat(ctx).or_else(|| deadline(ctx)).or_else(|| inbox(ctx))
}

fn react_to_events<S: State>(ctx: &mut NpcCtx, _: &mut S) -> Option<impl Action<S> + use<S>> {
    reaction_precedence(ctx, threat_reaction, deadline_reaction, inbox_reaction)
}

#[cfg(test)]
mod reaction_precedence_tests {
    use super::reaction_precedence;

    fn pending(_: &mut ()) -> Option<&'static str> { Some("pending") }
    fn absent(_: &mut ()) -> Option<&'static str> { None }

    /// T3.34's own contention case: threat and inbox both pending, no
    /// deadline — threat must win. Distinct payloads (not just `Some`)
    /// so the winner's IDENTITY, not merely its presence, is checked.
    #[test]
    fn threat_beats_pending_inbox() {
        fn threat(_: &mut ()) -> Option<&'static str> { Some("threat") }
        fn inbox(_: &mut ()) -> Option<&'static str> { Some("inbox") }
        assert_eq!(reaction_precedence(&mut (), threat, absent, inbox), Some("threat"));
    }

    /// T3.34's other contention case: deadline and inbox both pending, no
    /// threat — deadline must win.
    #[test]
    fn deadline_beats_pending_inbox_when_no_threat() {
        fn deadline(_: &mut ()) -> Option<&'static str> { Some("deadline") }
        fn inbox(_: &mut ()) -> Option<&'static str> { Some("inbox") }
        assert_eq!(reaction_precedence(&mut (), absent, deadline, inbox), Some("deadline"));
    }

    #[test]
    fn inbox_wins_only_when_nothing_else_pending() {
        assert_eq!(reaction_precedence(&mut (), absent, absent, pending), Some("pending"));
    }

    /// Non-vacuity: reproducing the OLD (pre-T3.34) inbox > threat >
    /// deadline order on the SAME contention case gives a DIFFERENT
    /// winner — proving this test actually discriminates between the two
    /// policies, not just that both compile and return `Some`.
    #[test]
    fn non_vacuous_against_the_old_accidental_order() {
        fn threat(_: &mut ()) -> Option<&'static str> { Some("threat") }
        fn inbox(_: &mut ()) -> Option<&'static str> { Some("inbox") }
        let new_order = reaction_precedence(&mut (), threat, absent, inbox);
        // The old code was `check_inbox().or_else(check_for_enemies).or_else(check_for_timeouts)`.
        let old_order = reaction_precedence(&mut (), inbox, threat, absent);
        assert_ne!(old_order, new_order, "the two policies must disagree on this contention case");
        assert_eq!(new_order, Some("threat"));
        assert_eq!(old_order, Some("inbox"));
    }
}

// T3.35+T3.39 (E3-WT): non-vacuity for `check_for_enemies`'s threat_policy
// wiring — proves real proximity now decides (previously canonical-Actor-
// order alone did), and that the DET-AIT-004 tiebreak survives exact ties.
#[cfg(test)]
mod pick_hostile_threat_tests {
    use super::pick_hostile_threat;
    use common::rtsim::{Actor, NpcId};
    use slotmap::KeyData;
    use vek::Vec3;

    fn actor(raw: u64) -> Actor { Actor::Npc(NpcId::from(KeyData::from_ffi(raw))) }

    #[test]
    fn no_candidates_is_none() {
        assert_eq!(pick_hostile_threat(Vec3::zero(), []), None);
    }

    /// Real proximity now decides: closer enemy wins even though its
    /// `Actor` id sorts higher than the farther one (so this can't be
    /// passing by accident of the old canonical-order tiebreak alone).
    #[test]
    fn closer_enemy_wins_even_with_a_higher_actor_id() {
        let wpos = Vec3::new(0.0, 0.0, 0.0);
        let far_low_id = (actor(1), Vec3::new(20.0, 0.0, 0.0));
        let close_high_id = (actor(9), Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(
            pick_hostile_threat(wpos, [far_low_id, close_high_id]),
            Some(actor(9))
        );
    }

    /// Exact distance tie: DET-AIT-004's canonical-order tiebreak is
    /// preserved via `threat_policy::compare`'s `tiebreak` term — higher
    /// `Actor` wins, independent of iteration order.
    #[test]
    fn exact_tie_resolves_by_actor_order_independently_of_iteration_order() {
        let wpos = Vec3::new(0.0, 0.0, 0.0);
        let a = (actor(3), Vec3::new(5.0, 0.0, 0.0));
        let b = (actor(7), Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(pick_hostile_threat(wpos, [a, b]), Some(actor(7)));
        assert_eq!(pick_hostile_threat(wpos, [b, a]), Some(actor(7)));
    }
}

fn humanoid() -> impl Action<DefaultState> {
    choose(|ctx, _, consider| {
        if let Some(riding) = &ctx.data.npcs.mounts.get_mount_link(ctx.npc_id) {
            if riding.is_steering {
                if let Some(vehicle) = ctx.data.npcs.get(riding.mount) {
                    match vehicle.body {
                        comp::Body::Ship(body @ comp::ship::Body::AirBalloon) => {
                            consider.important(pilot(body));
                        },
                        comp::Body::Ship(comp::ship::Body::DefaultAirship) => {
                            consider.important(airship_ai::pilot_airship());
                        },
                        comp::Body::Ship(
                            comp::ship::Body::SailBoat | comp::ship::Body::Galleon,
                        ) => {
                            consider.important(captain());
                        },
                        _ => {},
                    }
                } else {
                    consider.casual(finish());
                }
            } else {
                consider.important(
                    socialize().map_state(|state: &mut DefaultState| &mut state.socialize_timer),
                );
            }
        } else if let Some(job) = &ctx.npc.job {
            // NPCs should try to perform their jobs
            match job {
                Job::Hired(tgt, _) => {
                    if util::actor_exists(ctx, *tgt) {
                        consider.important(hired(*tgt));
                    } else {
                        ctx.controller.end_hiring();
                    }
                },
                Job::Quest(quest_id) => {
                    match ctx.data.quests.get(*quest_id).map(|q| &q.kind) {
                        // TODO: Support escort quests in which we are the escorter
                        Some(QuestKind::Escort {
                            escortee,
                            escorter,
                            to,
                        }) if *escortee == Actor::Npc(ctx.npc_id) => {
                            consider.important(quest::escorted(*quest_id, *escorter, *to));
                        },
                        // A quest job that can't be acted upon gets ended
                        _ => ctx.controller.end_quest(),
                    }
                },
            };
        } else {
            let action = match ctx.npc.profession() {
                Some(Profession::Adventurer(_) | Profession::Merchant) => {
                    adventure().l().l().l()
                },
                Some(Profession::Pirate(is_leader)) => pirate(is_leader).r().l().l(),
                _ => {
                    // bastion (IDLE-HOME-LEASH): colonists NEVER run the
                    // villager routine — their vanilla `home` is the nearest
                    // worldgen TOWN (a colony-spawn artifact), so "going
                    // home" walks them off-colony: the AUTON-2 idle drift.
                    // They orbit the colony anchor instead (painted Meeting
                    // zone beats first-stockpile centroid; no anchor yet →
                    // idle in place).
                    if ctx.npc.bastion_colonist.is_some() {
                        colonist_idle(ctx.data.bastion_home_anchor).r().l()
                    } else if let Some(home) = ctx.npc.home {
                        villager(home).l().r()
                    } else {
                        idle().r().r() // Homeless
                    }
                },
            };

            consider.casual(action);
        }
    })
    .interrupt_with(react_to_events)
}

fn bird_large() -> impl Action<DefaultState> {
    now(|ctx, bearing: &mut Vec2<f32>| {
        *bearing = bearing
            .map(|e| e + ctx.rng.random_range(-0.1..0.1))
            .try_normalized()
            .unwrap_or_default();
        let bearing_dist = 15.0;
        let mut pos = ctx.npc.wpos.xy() + *bearing * bearing_dist;
        let is_deep_water =
            matches!(ctx.npc.body, common::comp::Body::BirdLarge(b) if matches!(b.species, bird_large::Species::SeaWyvern))
                || ctx
                .world
                .sim()
                .get(pos.as_().wpos_to_cpos()).is_none_or(|c| {
                    c.alt - c.water_alt < -120.0 && (c.river.is_ocean() || c.river.is_lake())
                });
        if is_deep_water {
            *bearing *= -1.0;
            pos = ctx.npc.wpos.xy() + *bearing * bearing_dist;
        };
        // when high tree_density fly high, otherwise fly low-mid
        let npc_pos = ctx.npc.wpos.xy();
        let trees = ctx
            .world
            .sim()
            .get(npc_pos.as_().wpos_to_cpos()).is_some_and(|c| c.tree_density > 0.1);
        let height_factor = if trees {
            2.0
        } else {
            ctx.rng.random_range(0.4..0.9)
        };

        // without destination site fly to next waypoint
        let mut dest_site = pos;
        if let Some(home) = ctx.npc.home {
            let is_home = ctx.npc.current_site == Some(home);
            if is_home {
                if let Some((id, _)) = ctx.data
                    .sites
                    .iter()
                    .filter(|(id, site)| {
                        *id != home
                            && site.world_site.is_some_and(|site| {
                            match ctx.npc.body {
                                common::comp::Body::BirdLarge(b) => match b.species {
                                    bird_large::Species::Phoenix => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::Terracotta
                                    | SiteKind::Haniwa
                                    | SiteKind::Myrmidon
                                    | SiteKind::Adlet
                                    | SiteKind::DwarvenMine
                                    | SiteKind::ChapelSite
                                    | SiteKind::Cultist
                                    | SiteKind::Gnarling
                                    | SiteKind::Sahagin
                                    | SiteKind::VampireCastle)),
                                    bird_large::Species::Cockatrice => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::GiantTree)),
                                    bird_large::Species::Roc => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::Haniwa
                                    | SiteKind::Cultist)),
                                    bird_large::Species::FlameWyvern => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::DwarvenMine
                                    | SiteKind::Terracotta)),
                                    bird_large::Species::CloudWyvern => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::ChapelSite
                                    | SiteKind::Sahagin)),
                                    bird_large::Species::FrostWyvern => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::Adlet
                                    | SiteKind::Myrmidon)),
                                    bird_large::Species::SeaWyvern => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::ChapelSite
                                    | SiteKind::Sahagin)),
                                    bird_large::Species::WealdWyvern => matches!(ctx.index.sites.get(site).kind,
                                    Some(SiteKind::GiantTree
                                    | SiteKind::Gnarling)),
                                },
                                _ => matches!(&ctx.index.sites.get(site).kind, Some(SiteKind::GiantTree)),
                            }
                        })
                    })
                    /*choose closest destination:
                    .min_by_key(|(_, site)| site.wpos.as_().distance(npc_pos) as i32)*/
                //choose random destination:
                .choose(&mut ctx.rng)
                {
                    ctx.controller.set_new_home(id)
                }
            } else if let Some(site) = ctx.data.sites.get(home) {
                dest_site = site.wpos.as_::<f32>()
            }
        }
        goto_2d_flying(
            pos,
            0.2,
            bearing_dist,
            8.0,
            8.0,
            ctx.npc.body.flying_height() * height_factor,
        )
            // If we are too far away from our waypoint position we can stop since we aren't going to a specific place.
            // If waypoint position is further away from destination site find a new waypoint
            .stop_if(move |ctx: &mut NpcCtx| {
                ctx.npc.wpos.xy().distance_squared(pos) > (bearing_dist + 5.0).powi(2)
                    || dest_site.distance_squared(pos) > dest_site.distance_squared(npc_pos)
            })
            // If waypoint position wasn't reached within 10 seconds we're probably stuck and need to find a new waypoint.
            .stop_if(timeout(10.0))
            .debug({
                let bearing = *bearing;
                move || format!("Moving with a bearing of {:?}", bearing)
            })
    })
        .repeat()
        .with_state(Vec2::<f32>::zero())
        .map(|_, _| ())
}

fn monster() -> impl Action<DefaultState> {
    now(
        |ctx,
         (bearing, roam_location, roam_location_timestamp): &mut (
            Vec2<f32>,
            Option<Vec2<f32>>,
            Time,
        )| {
            // Some NPC's (like Frost Gigas) can roam the world, and thus need
            // to periodically choose a new random location to roam towards.
            // This is particularly important for quests, as it makes quest NPC
            // waypoints a bit more reliable by keeping an NPC in a similar
            // location for a little while. There is otherwise nothing special
            // about the roam location itself.
            //
            // Choose a new roam location once every 10 minutes (+/- 1 minute)
            // or so.
            let desired_roam_location = match *roam_location {
                Some(_)
                    if ctx.time
                        > roam_location_timestamp
                            .add_seconds((540 + ctx.rng.random_range(0..120)) as f64) =>
                {
                    None
                },
                Some(rl) => Some(rl),
                _ => None,
            }
            .unwrap_or_else(|| {
                *roam_location_timestamp = ctx.time;
                ctx.npc
                    .wpos
                    .xy()
                    .map(|e| e + ctx.rng.random_range(-500.0..500.0))
            });

            *roam_location = Some(desired_roam_location);

            // Tend to want to move back towards the roam location
            *bearing += (desired_roam_location - ctx.npc.wpos.xy()) * ctx.dt;
            *bearing = bearing
                .map(|e| e + ctx.rng.random_range(-1.0..1.0) * ctx.dt)
                .try_normalized()
                .unwrap_or_default();
            let bearing_dist = 24.0;
            let mut pos = ctx.npc.wpos.xy() + *bearing * bearing_dist;
            let is_deep_water = ctx
                .world
                .sim()
                .get(pos.as_().wpos_to_cpos())
                .is_none_or(|c| {
                    c.alt - c.water_alt < -10.0 && (c.river.is_ocean() || c.river.is_lake())
                });
            if !is_deep_water {
            goto_2d(pos, 0.7, 8.0)
        } else {
            *bearing *= -1.0;

            pos = ctx.npc.wpos.xy() + *bearing * 24.0;

            goto_2d(pos, 0.7, 8.0)
        }
        // If we are too far away from our goal position we can stop since we aren't going to a specific place.
        .stop_if(move |ctx: &mut NpcCtx| {
            ctx.npc.wpos.xy().distance_squared(pos) > (bearing_dist + 5.0).powi(2)
        })
        .debug({
            let bearing = *bearing;
            move || format!("Moving with a bearing of {:?}", bearing)
        })
        },
    )
    .repeat()
    .with_state((Vec2::<f32>::zero(), None, Time(0.0)))
    .map(|_, _| ())
}

fn think() -> impl Action<DefaultState> {
    now(|ctx, _| match ctx.npc.body {
        common::comp::Body::Humanoid(_) => humanoid().l().l().l(),
        common::comp::Body::BirdLarge(_) => bird_large().r().l().l(),
        _ => match &ctx.npc.role {
            Role::Civilised(_) => socialize()
                .map_state(|state: &mut DefaultState| &mut state.socialize_timer)
                .l()
                .r()
                .l(),
            Role::Monster => monster().r().r().l(),
            Role::Wild => idle().r(),
            Role::Vehicle => idle().r(),
        },
    })
}
