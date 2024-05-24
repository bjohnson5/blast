``` mermaid
sequenceDiagram
    blast_cli->>blast_core: create_network("network_name", model_map)

    blast_cli->>blast_core: start_network()
    blast_core->>blast_model_manager: start_model(model_name)
    blast_core->>blast_model_manager: start_nodes(model_name, num_nodes)
    blast_model_manager->>model: start_nodes()
    blast_model_manager->>model: get_sim_ln()
    model-->>blast_model_manager: simln_json_data
    blast_model_manager-->>blast_core: simln_json_data
    blast_core->>blast_simln_manager: add_nodes(simln_json_data)

    blast_cli->>blast_core: list_peers(node_id)
    blast_core->>blast_model_manager: list_peers(node_id)
    blast_model_manager->>model: list_peers(node_id)
    model->>blast_model_manager: peers<String>[]
    blast_model_manager-->>blast_core: peers<String>[]
    blast_core-->>blast_cli: peers<String>[]

    blast_cli->>blast_core: wallet_balance(node_id)
    blast_core->>blast_model_manager: wallet_balance(node_id)
    blast_model_manager->>model: wallet_balance(node_id)
    model->>blast_model_manager: balance
    blast_model_manager-->>blast_core: balance
    blast_core-->>blast_cli: balance

    blast_cli->>blast_core: open_channel(node_id, node_id2, amount, push, confirm)
    blast_core->>blast_model_manager: open_channel(node_id, node_id2, amount, push)
    blast_model_manager->>model: open_channel()
    model->>blast_model_manager: success
    blast_model_manager-->>blast_core: success
    blast_core-->>blast_cli: success
    
    blast_cli->>blast_core: add_activity("nodeid1", "nodeid2", 0, None, 1, 2000)
    blast_core->>blast_simln_manager: add_activity("nodeid1", "nodeid2", start, count, interval, amount)

    blast_cli->>blast_core: add_event("CloseChannel", id)
    blast_core->>blast_event_manager: add_event("CloseChannel", id)

    blast_cli->>blast_core: finalize_simulation()
    blast_core->>blast_simln_manager: setup_simln()
    blast_simln_manager->>simln: Simulation::new()

    blast_cli->>blast_core: start_simulation()
    blast_core->>blast_simln_manager: start()
    blast_simln_manager->>simln: run()
    blast_core->>blast_event_manager: start()
    blast_core->>blast_model_manager: process_events()

    blast_cli->>blast_core: stop_simulation()
    blast_core->>blast_simln_manager: stop()
    blast_simln_manager->>simln: shutdown()
    blast_core->>blast_event_manager: stop()

    blast_cli->>blast_core: stop_network()
    blast_core->>blast_model_manager: stop_model()
    blast_model_manager->>model: stop_model()
```