use crate::client::Client;
use common::{
    comp::invite::{Invite, PendingInvites},
    resources::Time,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{InviteAnswer, ServerGeneral};
use specs::{Entities, Join, Read, ReadStorage, WriteStorage};

/// This system removes timed out invites
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Invite>,
        WriteStorage<'a, PendingInvites>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Uid>,
        Read<'a, Time>,
    );

    const NAME: &'static str = "invite_timeout";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, mut invites, mut pending_invites, clients, uids, time): Self::SystemData,
    ) {
        // DET-ADD-002: compare against the deterministic sim-clock (seconds),
        // not wall-clock Instant::now().
        let now = time.0;
        let timed_out_invites = (&entities, &invites)
            .join()
            .filter_map(|(invitee, Invite { inviter, kind })| {
                // Retrieve timeout invite from pending invites
                let pending = &mut pending_invites.get_mut(*inviter)?.0;
                let index = pending.iter().position(|p| p.0 == invitee)?;

                // Stop if not timed out
                if pending[index].2 > now {
                    return None;
                }

                // Remove pending entry
                pending.swap_remove(index);

                // If no pending invites remain remove the component
                if pending.is_empty() {
                    pending_invites.remove(*inviter);
                }

                // Inform inviter of timeout
                if let (Some(client), Some(target)) =
                    (clients.get(*inviter), uids.get(invitee).copied())
                {
                    client.send_fallible(ServerGeneral::InviteComplete {
                        target,
                        answer: InviteAnswer::TimedOut,
                        kind: *kind,
                    });
                }

                Some(invitee)
            })
            .collect::<Vec<_>>();

        for entity in timed_out_invites {
            invites.remove(entity);
        }
    }
}
