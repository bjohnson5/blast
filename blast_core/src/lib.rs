
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::collections::HashMap;

use bitcoin::secp256k1::PublicKey;
use tokio::sync::Mutex;

use sim_lib::ActivityDefinition;
use sim_lib::Simulation;
use sim_lib::LightningNode;

use blast_model_interface::BlastModelInterface;

pub struct Blast {
    blast_model_interface: BlastModelInterface,
    simln: Option<Simulation>
}

impl Blast {
    pub fn new() -> Self {
        let blast = Blast {
            blast_model_interface: BlastModelInterface::new(),
            simln: None
        };

        blast
    }

    pub fn run_simulation(&mut self) {
        // TODO: need to get all the SimLn data from all nodes and compile it into one
        // TODO: part of the BLAST RPC protocol will include a GetSimLN function that models will have to implement
        let clients: HashMap<PublicKey, Arc<Mutex<dyn LightningNode>>> = HashMap::new();
        let validated_activities: Vec<ActivityDefinition> = Vec::new();
        let sim = Simulation::new(
            clients,
            validated_activities,
            None,
            1,
            0.1,
            None,
        );
        self.simln = Some(sim);
        //sim.run().await?;
    }

    pub fn stop_simulation(&mut self) {
        match &self.simln {
            Some(s) => {
                s.shutdown();
            },
            None => {}
        };
    }
    
    pub async fn create_nodes(&mut self, model: String, num_nodes: u64, running: Arc<AtomicBool>) -> Option<Child> {
        let blast_lnd = self.blast_model_interface.create_nodes(model, num_nodes, running).await;
        blast_lnd
    }

    pub async fn list_peers(&mut self, node_id: String) {
        match self.blast_model_interface.list_peers(node_id).await {
            Ok(_) => {
            },
            Err(_) => {
                println!("Error calling list peers")
            }
        }
    }

    pub async fn get_info(&mut self, node_id: String) {
        match self.blast_model_interface.get_info(node_id).await {
            Ok(_) => {
            },
            Err(_) => {
                println!("Error calling get_info")
            }
        }
    }
}
