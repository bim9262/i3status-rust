use ::pipewire::{keys, properties, spa::ReadableDict, types::ObjectType, Context, MainLoop};
use tokio::sync::mpsc::{self, Receiver, Sender};

use std::sync::Mutex;
use std::{collections::HashMap, thread};

use super::*;

static CLIENT: Lazy<Result<Client>> = Lazy::new(Client::new);
static NODES: Lazy<Mutex<HashMap<u32, Node>>> = Lazy::new(default);
static LINKS: Lazy<Mutex<HashMap<u32, Link>>> = Lazy::new(default);

#[derive(Debug)]
struct Node {
    node_name: String,
    media_class: Option<String>,
    media_role: Option<String>,
}
#[derive(Debug)]
struct Link {
    link_output_node: u32,
    link_input_node: u32,
}

#[derive(Deserialize, Debug, SmartDefault)]
#[serde(rename_all = "lowercase", deny_unknown_fields, default)]
pub struct Config {
    exclude_output: Vec<String>,
    exclude_input: Vec<String>,
}

#[derive(Default)]
struct Client {
    event_listeners: Mutex<Vec<Sender<()>>>,
}

impl Client {
    fn new() -> Result<Client> {
        thread::Builder::new()
            .name("privacy_pipewire".to_string())
            .spawn(move || {
                let proplist = properties! {*keys::APP_NAME => env!("CARGO_PKG_NAME")};

                let mainloop = MainLoop::new().expect("Failed to create main loop");

                let context = Context::with_properties(&mainloop, proplist)
                    .expect("Failed to create context");
                let core = context.connect(None).expect("Failed to connect");
                let registry = core.get_registry().expect("Failed to get registry");

                // Register a callback to the `global` event on the registry, which notifies of any new global objects
                // appearing on the remote.
                // The callback will only get called as long as we keep the returned listener alive.
                let _listener = registry
                    .add_listener_local()
                    .global(move |global| {
                        if let Some(global_props) = &global.props {
                            if global.type_ == ObjectType::Node {
                                NODES.lock().unwrap().insert(
                                    global.id,
                                    Node {
                                        node_name: global_props.get(&keys::NODE_NICK).map_or_else(
                                            || {
                                                global_props.get(&keys::NODE_NAME).map_or_else(
                                                    || format!("node_{}", global.id),
                                                    |s| s.to_string(),
                                                )
                                            },
                                            |s| s.to_string(),
                                        ),
                                        media_class: global_props
                                            .get(&keys::MEDIA_CLASS)
                                            .map(|s| s.to_string()),
                                        media_role: global_props
                                            .get(&keys::MEDIA_ROLE)
                                            .map(|s| s.to_string()),
                                    },
                                );
                                CLIENT.as_ref().unwrap().send_update_event();
                            } else if global.type_ == ObjectType::Link {
                                if let (Some(link_output_node), Some(link_input_node)) = (
                                    global_props
                                        .get(&keys::LINK_OUTPUT_NODE)
                                        .and_then(|s| s.parse().ok()),
                                    global_props
                                        .get(&keys::LINK_INPUT_NODE)
                                        .and_then(|s| s.parse().ok()),
                                ) {
                                    LINKS.lock().unwrap().insert(
                                        global.id,
                                        Link {
                                            link_output_node,
                                            link_input_node,
                                        },
                                    );
                                    CLIENT.as_ref().unwrap().send_update_event();
                                }
                            }
                        }
                    })
                    .global_remove(move |uid| {
                        NODES.lock().unwrap().remove(&uid);
                        LINKS.lock().unwrap().remove(&uid);
                        CLIENT.as_ref().unwrap().send_update_event();
                    })
                    .register();

                mainloop.run();
            })
            .error("failed to spawn a thread")?;

        Ok(Client::default())
    }

    fn add_event_listener(&self, tx: Sender<()>) {
        self.event_listeners.lock().unwrap().push(tx);
    }

    fn send_update_event(&self) {
        self.event_listeners
            .lock()
            .unwrap()
            .retain(|tx| tx.blocking_send(()).is_ok());
    }
}

pub(super) struct Monitor<'a> {
    config: &'a Config,
    updates: Receiver<()>,
}

impl<'a> Monitor<'a> {
    pub(super) async fn new(config: &'a Config) -> Result<Self> {
        let client = CLIENT.as_ref().error("Could not get client")?;

        let (tx, rx) = mpsc::channel(32);
        client.add_event_listener(tx);
        Ok(Self {
            config,
            updates: rx,
        })
    }
}

#[async_trait]
impl<'a> PrivacyMonitor for Monitor<'a> {
    async fn get_info(&mut self) -> Result<PrivacyInfo> {
        let mut mapping: PrivacyInfo = PrivacyInfo::new();

        for Link {
            link_output_node,
            link_input_node,
            ..
        } in LINKS.lock().unwrap().values()
        {
            let nodes = NODES.lock().unwrap();
            if let (Some(output_node), Some(input_node)) =
                (nodes.get(link_output_node), nodes.get(link_input_node))
            {
                if input_node.media_class != Some("Audio/Sink".into())
                    && !self.config.exclude_output.contains(&output_node.node_name)
                    && !self.config.exclude_input.contains(&input_node.node_name)
                {
                    let type_ = if input_node.media_class == Some("Stream/Input/Video".into()) {
                        if output_node.media_role == Some("Camera".into()) {
                            Type::Webcam
                        } else {
                            Type::Video
                        }
                    } else if input_node.media_class == Some("Stream/Input/Audio".into()) {
                        if output_node.media_class == Some("Audio/Sink".into()) {
                            Type::AudioSink
                        } else {
                            Type::Audio
                        }
                    } else {
                        Type::Unknown
                    };
                    mapping
                        .entry(type_)
                        .or_default()
                        .entry(output_node.node_name.clone())
                        .or_default()
                        .insert(input_node.node_name.clone());
                }
            }
        }

        Ok(mapping)
    }

    async fn wait_for_change(&mut self) -> Result<()> {
        self.updates.recv().await;
        Ok(())
    }
}
