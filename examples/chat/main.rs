use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io;
use std::time::Duration;

use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, noise, ping, yamux, SwarmBuilder};

use tracing_error::ErrorLayer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;

/// Initializes a tracing Subscriber for logging
#[allow(dead_code)]
pub fn init_tracing_subscriber() {
    tracing_subscriber::Registry::default()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(ErrorLayer::default())
        .with(tracing_subscriber::fmt::layer())
        .init()
}

#[derive(NetworkBehaviour)]
struct CustomBehaviour {
    gossipsub: gossipsub::Behaviour,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing_subscriber();

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            let message_auth = gossipsub::MessageAuthenticity::Signed(key.clone());

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
                .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

            let gossipsub = gossipsub::Behaviour::new(message_auth, gossipsub_config)?;

            Ok(CustomBehaviour { gossipsub })
        })?
        .build();

    Ok(())
}
