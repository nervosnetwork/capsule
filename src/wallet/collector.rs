use super::cli_types::{Address, HumanCapacity, LiveCell, LiveCellInfo, LiveCellInfoVec};
use super::util::handle_cmd;
use ckb_tool::ckb_types::{core::Capacity, packed::*};
use log::{debug, trace};
use std::collections::HashSet;
use std::process::Command;

pub struct Collector {
    locked_cells: HashSet<OutPoint>,
    ckb_cli_bin: String,
    api_uri: String,
}

impl Collector {
    pub fn new(api_uri: String, ckb_cli_bin: String) -> Self {
        Collector {
            locked_cells: HashSet::default(),
            api_uri,
            ckb_cli_bin,
        }
    }

    pub fn lock_cell(&mut self, out_point: OutPoint) {
        self.locked_cells.insert(out_point);
    }

    pub fn is_live_cell_locked(&self, live_cell: &LiveCell) -> bool {
        self.locked_cells.contains(&live_cell.out_point())
    }

    pub fn collect_live_cells(&self, address: Address, capacity: Capacity) -> HashSet<LiveCell> {
        const BLOCKS_IN_BATCH: u64 = 1000;
        const LIMIT: u64 = 2000;

        let tip_number = self.get_tip_block_number();
        debug!(
            "collect live cells: target {} address {} tip_number {}",
            capacity, address, tip_number
        );
        let mut live_cells = HashSet::new();
        let mut collected_capacity = 0;
        for i in 0.. {
            let from = i * BLOCKS_IN_BATCH;
            if from > tip_number {
                panic!(
                    "can't find enough live cells, found {} expected {}",
                    HumanCapacity::from(collected_capacity),
                    HumanCapacity::from(capacity.as_u64())
                );
            }
            let to = (i + 1) * BLOCKS_IN_BATCH;
            let cells = self.get_live_cells_by_lock_hash(address.clone(), from, to, LIMIT);
            trace!("get cells: from {} to {} cells {:?}", from, to, cells.len());
            if cells.is_empty() {
                continue;
            }
            let iter = cells
                .into_iter()
                .filter(|cell| cell.data_bytes == 0 && cell.type_hashes.is_none());
            for cell in iter {
                let cell: LiveCell = cell.into();
                // cell is in use, but not yet committed
                if self.is_live_cell_locked(&cell) {
                    continue;
                }
                let cell_capacity = cell.capacity;
                if !live_cells.insert(cell) {
                    // skip collected cell
                    continue;
                }
                collected_capacity += cell_capacity;
                if collected_capacity > capacity.as_u64() {
                    break;
                }
            }
            if collected_capacity > capacity.as_u64() {
                break;
            }
        }
        live_cells
    }

    fn get_tip_block_number(&self) -> u64 {
        let output = handle_cmd(
            Command::new(&self.ckb_cli_bin)
                .arg("--url")
                .arg(&self.api_uri)
                .arg("rpc")
                .arg("--wait-for-sync")
                .arg("get_tip_block_number")
                .arg("--output-format")
                .arg("json")
                .output()
                .expect("run cmd"),
        )
        .expect("run cmd error");
        trace!(
            "parse ckb-cli output: {}",
            String::from_utf8(output.clone()).unwrap()
        );
        let tip_block_number: u64 = serde_json::from_slice(&output).expect("parse resp");
        tip_block_number
    }

    fn get_live_cells_by_lock_hash(
        &self,
        address: Address,
        from: u64,
        to: u64,
        limit: u64,
    ) -> Vec<LiveCellInfo> {
        let output = handle_cmd(
            Command::new(&self.ckb_cli_bin)
                .arg("--url")
                .arg(&self.api_uri)
                .arg("wallet")
                .arg("--wait-for-sync")
                .arg("get-live-cells")
                .arg("--address")
                .arg(address.display_with_network(address.network()))
                .arg("--from")
                .arg(format!("{}", from))
                .arg("--to")
                .arg(format!("{}", to))
                .arg("--limit")
                .arg(format!("{}", limit))
                .arg("--output-format")
                .arg("json")
                .output()
                .expect("run cmd"),
        )
        .expect("run cmd error");
        trace!(
            "parse ckb-cli output: {}",
            String::from_utf8(output.clone()).unwrap()
        );
        let resp: LiveCellInfoVec = serde_json::from_slice(&output).expect("parse resp");
        resp.live_cells
    }
}
