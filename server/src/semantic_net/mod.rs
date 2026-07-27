//! `APEX-T3.3.11+`: the server-side canonical semantic-send pipeline
//! (outbox, and later the total-order registries and egress owner).
//! Spec: `PROJECT-BASTION-APEX-MICROSTEP-APEX-T3.3-SEMANTIC-NET-ENVELOPE.md`.

pub mod order;
pub mod outbox;
