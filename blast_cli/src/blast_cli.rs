use crate::shared::*;
use crate::new::*;
use crate::load::*;
use crate::configure::*;
use crate::run::*;

pub struct BlastCli {
    pub new: NewTab,
    pub load: LoadTab,
    pub config: ConfigureTab,
    pub run: RunTab,
}

impl BlastCli {
    pub fn new() -> Self {
        // TODO: this is a placeholder, initialize with the actual available models
        let mut model_list: Vec<Model> = Vec::new();
        model_list.push(Model{name: String::from("blast_lnd"), num_nodes: 0});
        model_list.push(Model{name: String::from("blast_ldk"), num_nodes: 0});
        model_list.push(Model{name: String::from("blast_cln"), num_nodes: 0});

        // TODO: this is a placeholder, initialize with the actual saved simulations
        let mut sim_list: Vec<String> = Vec::new();
        sim_list.push(String::from("Test Simulation 1"));
        sim_list.push(String::from("Another Test Simulation"));
        sim_list.push(String::from("Simulation3"));

        let nt = NewTab{models: StatefulList::with_items(model_list)};
        let lt = LoadTab{sims: StatefulList::with_items(sim_list)};
        let ct = ConfigureTab::new();
        let rt = RunTab::new();

        Self {
            new: nt,
            load: lt,
            config: ct,
            run: rt,
        }
    }
}
