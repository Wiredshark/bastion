# APEX-T2.5 rollout block — open items

T2.5 closed MECHANISM-COMPLETE / ROLLOUT-GATED. Nothing below may go live
until this block clears; the gate blocks rollout, not the mechanism.

- [ ] .04b deployment evidence: archive inventory, real base_content_root
      identity, base command inventory (base_resources currently empty).
- [ ] Operator-reviewed PRODUCTION admission policy (values, conflict
      decisions). Until it exists, production admission is fail-closed.
- [ ] .23 late-load API removal + RequestPlugins/PluginData retirement
      (Fable's ruling: one deliberate rollout block, not incremental).
- [ ] F2 (T2.5 elevated review, MED): update_skeleton still last-wins while
      create_body routes skeleton_owners — ownership can diverge between
      create and update. FIX = option (a): owner-tagged body handle
      (bindgen resource identity + Voxygen call-site migration). Option (b)
      interim lookup DECLINED as throwaway. Sits here with its cross-crate
      siblings.
- [ ] VM fixtures: fuel/memory trap, lifecycle trap-ordering, live wasm
      registration, world-baseline permutation, governed-failure-at-boot.
