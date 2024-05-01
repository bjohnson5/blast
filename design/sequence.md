``` mermaid
sequenceDiagram
    blast_cli->>blast_core: import_graph("net.json")
    blast_cli->>blast_core: add_node(BlastNode{})
    blast_cli->>blast_core: add_channel(BlastChannel{})
    blast_cli->>blast_core: create_event(BlastOpenChannelEvent{}, 2s)
    blast_cli->>blast_core: create_event(BlastCloseChannelEvent{}, 15s)
    blast_cli->>blast_core: load_simulation()
    blast_core->>blast_model_interface: start_models()
    blast_core->>blast_model_interface: start_nodes("blast_lnd", 100)
    blast_model_interface->>model: start_nodes()
    blast_model_interface->>model: get_sim_ln()
    model-->>blast_model_interface: simln json data
    blast_model_interface-->>blast_core: simln json data
    blast_core->>blast_core: setup_simln()
    blast_core->>simln: Simulation::new()
    simln-->>blast_core: Simulation
    blast_core-->>blast_cli: model_processes<BlastModel>[]

    blast_cli->>blast_core: list_peers(node_id)
    blast_core->>blast_model_interface: list_peers(node_id)
    blast_model_interface-->>blast_core: peers<String>[]
    blast_core-->>blast_cli: peers<String>[]

    blast_cli->>blast_core: wallet_balance(node_id)
    blast_core->>blast_model_interface: wallet_balance(node_id)
    blast_model_interface-->>blast_core: balance
    blast_core-->>blast_cli: balance

    blast_cli->>blast_core: start_simulation()
    blast_core->>simln: run()

    blast_cli->>blast_core: stop_simulation()
    blast_core->>simln: shutdown()
```