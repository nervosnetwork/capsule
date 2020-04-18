use ckb_tool::ckb_jsonrpc_types::LiveCell;
use ckb_tool::ckb_types::{core::Capacity, packed::*, prelude::*};
use ckb_tool::rpc_client::RpcClient;
use std::collections::HashSet;

pub struct Collector {
    locked_cells: HashSet<OutPoint>,
}

impl Collector {
    pub fn new() -> Self {
        Collector {
            locked_cells: HashSet::default(),
        }
    }

    pub fn lock_cell(&mut self, out_point: OutPoint) {
        self.locked_cells.insert(out_point);
    }

    pub fn is_live_cell_locked(&self, live_cell: &LiveCell) -> bool {
        let index: u64 = live_cell.created_by.index.into();
        let out_point = OutPoint::new_builder()
            .tx_hash(live_cell.created_by.tx_hash.pack())
            .index((index as u32).pack())
            .build();
        self.locked_cells.contains(&out_point)
    }

    pub fn collect_live_cells(
        &self,
        rpc_client: &RpcClient,
        lock_hash: Byte32,
        capacity: Capacity,
    ) -> Vec<LiveCell> {
        const PER_PAGE: u64 = 20u64;

        let mut live_cells = Vec::new();
        let mut collected_capacity = 0;
        for i in 0.. {
            let cells =
                rpc_client.get_live_cells_by_lock_hash(lock_hash.clone(), i as u64, PER_PAGE, None);
            if cells.is_empty() {
                panic!("can't find enough live cells");
            }
            let iter = cells.into_iter().filter(|cell| {
                cell.output_data_len.value() == 0 && cell.cell_output.type_.is_none()
            });
            for cell in iter {
                // cell is in use, but not yet committed
                if self.is_live_cell_locked(&cell) {
                    continue;
                }
                let cell_capacity = cell.cell_output.capacity.value();
                live_cells.push(cell);
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
}
