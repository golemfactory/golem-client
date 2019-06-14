macro_rules! map_statuses {
    {
        on($c:expr, $m:expr, $s:expr);
        $($component:expr => {
            $($method:expr => {
               $($stage:expr => $msg:expr),+
            }),+
        }),*
    } => {
            match $c {
                $(
                    $component => match $m {
                        $(
                            $method => match $s {
                                $(
                                    $stage => $msg,
                                    _ => "Unknown status"
                                ),+
                            }
                            _ => "Unknown method"
                        ),+
                    }
                    _ => "Unknown component"
                ),*
            }
        };
}


pub fn map_statuses(component: &str, method: &str, stage: &str) -> &'static str {
    map_statuses! {
        on(component, method, stage);
        "hyperdrive" => {
            "instance.check" => {
                "pre" => "Checking for hyperg",
                "post" => "Hyperg is available",
                "exception" => "Hyperg is not available"
            },
            "instance.connect" => {
                "pre" => "Connecting to hyperg",
                "post" => "Connected to hyperg",
                "exception" => "Cannot connect to hyperg"
            },
            "instance.version" => {
                "pre" => "Hyperg version checking",
                "post" => "Hyperg version checked",
                "exception" => "Outdated hyperg version"
            }
        },
        "docker" => {
            "instance.start" => {
                "pre" => "Docker is starting",
                "post" => "Docker is started",
                "exception" => "Error starting docker"
            },
            "instance.stop" => {
                "pre" => "Docker is stopping",
                "post" => "Docker is stopped",
                "exception" => "Error stopping docker"
            },
            "instance.check" => {
                "pre" => "Checking for docker",
                "post" => "Docker is available",
                "exception" => "Docker is not available"
            },
            "instance.env" => {
                "pre" => "Setting VM environment",
                "post" => "VM environment configured",
                "exception" => "Docker environment error"
            },
            "images.pull" => {
                "pre" => "Pulling Docker images",
                "post" => "Docker images downloaded",
                "exception" => "Error pulling Docker images"
            },
            "images.build" => {
                "pre" => "Building Docker images",
                "post" => "Docker images built",
                "exception" => "Error building Docker images"
            },
            "allocation" => {
                "exception" => "Resource allocation error"
            }
        },
         "hypervisor" => {
            "vm.create" => {
                "pre" => "Creating Docker VM",
                "post" => "Docker VM created",
                "exception" => "Error creating Docker VM"
            },
            "vm.restart" => {
                "pre" => "Restarting Docker VM",
                "post" => "Docker VM restarted",
                "exception" => "Error restarting Docker VM"
            },
            "vm.recover" => {
                "pre" => "Recovering Docker VM",
                "post" => "Docker VM recovered",
                "exception" => "Error recovering Docker VM"
            },
            "vm.stop" => {
                "pre" => "Creating Docker VM",
                "post" => "Docker VM created",
                "exception" => "Error stopping a VM"
            },
            "instance.check" => {
                "pre" => "Checking for Docker VM",
                "post" => "Docker VM is available",
                "exception" => "Docker VM is not available"
            }
        },
          "ethereum" => {
            "node.start" => {
                "pre" => "Connecting geth",
                "post" => "Geth connected",
                "exception" => "Error connecting geth"
            },
            "node.stop" => {
                "pre" => "Stopping geth",
                "post" => "Geth stopped",
                "exception" => "Error stopping geth"
            },
            "sync" => {
                "pre" => "Syncing chain",
                "post" => "Chain synced",
                "exception" => "Chain sync error"
            }
        },
        "client" => {
            "get_password" => {
                "pre" => "Requires password",
                "post" => "Logged In",
                "exception" => "Problem with password"
            },
            "new_password" => {
                "pre" => "Requires new password",
                "post" => "Registered",
                "exception" => "Problem with password"
            },
            "sync" => {
                "pre" => "Syncing Golem",
                "post" => "Golem synced",
                "exception" => "Error syncing Golem"
            },
            "start" => {
                "pre" => "Starting Golem",
                "post" => "Connecting to network",
                "exception" => "Error starting Golem"
            },
            "stop" => {
                "pre" => "Stopping Golem",
                "post" => "Golem has stopped",
                "exception" => "Error stopping Golem"
            },
            "quit" => {
                "pre" => "Terminating Golem",
                "post" => "Golem terminated",
                "exception" => "Error terminating Golem"
            }
        }
    }
}