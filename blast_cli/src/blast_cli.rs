// Blast libraries
use blast_core::Blast;

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
    pub blast: Blast
}

impl BlastCli {
    pub fn new() -> Self {
        // Create the blast core object
        let blast = Blast::new();

        let mut model_list: Vec<Model> = Vec::new();
        match blast.get_available_models() {
            Ok(models) => {
                for model_name in models {
                    model_list.push(Model{name: model_name.clone(), num_nodes: 0});
                }
            },
            Err(_) => {}
        }

        let mut sim_list: Vec<String> = Vec::new();
        match blast.get_available_sims() {
            Ok(sims) => {
                for name in sims {
                    sim_list.push(name);
                }
            },
            Err(_) => {}
        }

        let nt = NewTab{models: StatefulList::with_items(model_list)};
        let lt = LoadTab{sims: StatefulList::with_items(sim_list)};
        let ct = ConfigureTab::new();
        let rt = RunTab::new();

        Self {
            new: nt,
            load: lt,
            config: ct,
            run: rt,
            blast: blast
        }
    }
}