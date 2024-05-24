``` mermaid
sequenceDiagram
    blast_event_manager->>blast_model_manager: OPEN_CHANNEL_EVENT
    blast_model_manager->>model: open_channel()
    blast_event_manager->>blast_core: mine_blocks(BLOCKS_PER_FRAME)
    blast_event_manager->>tokio: sleep(FRAME_RATE)
```