use windows::{
    Win32::{
        Foundation::LUID,
        System::Performance::{
            PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER,
            PdhCollectQueryData, PdhGetFormattedCounterValue,
        },
    },
    core::Result,
};

use crate::pdh::{PDH_FUNCTION, PerfQueryHandle, add_perf_counters};

pub struct PerfTracker {
    query_handle: PerfQueryHandle,
    counter_handles: Vec<PDH_HCOUNTER>,
}

impl PerfTracker {
    pub fn new(process_id: u32, luid: Option<LUID>) -> Result<Self> {
        let counter_path = if let Some(luid) = luid {
            format!(
                r#"\GPU Engine(pid_{}_luid_{:#010X}_{:#010X}*engtype_3D)\Utilization Percentage"#,
                process_id, luid.HighPart, luid.LowPart,
            )
        } else {
            format!(
                r#"\GPU Engine(pid_{}*engtype_3D)\Utilization Percentage"#,
                process_id
            )
        };
        println!("Search path: {}", counter_path);

        let query_handle = PerfQueryHandle::open_query()?;
        let counter_handles = add_perf_counters(&query_handle, &counter_path)?;

        Ok(Self {
            query_handle,
            counter_handles,
        })
    }

    pub fn start(&self) -> Result<()> {
        self.collect_query_data()
    }

    pub fn get_current_value(&self) -> Result<f64> {
        self.collect_query_data()?;

        let mut utilization_value = 0.0;
        for counter_handle in &self.counter_handles {
            let counter_value = unsafe {
                let mut counter_type = 0;
                let mut counter_value = PDH_FMT_COUNTERVALUE::default();
                PDH_FUNCTION(PdhGetFormattedCounterValue(
                    *counter_handle,
                    PDH_FMT_DOUBLE,
                    Some(&mut counter_type),
                    &mut counter_value,
                ))
                .ok()?;
                counter_value
            };
            assert_eq!(counter_value.CStatus, PDH_CSTATUS_VALID_DATA);
            let value = unsafe { counter_value.Anonymous.doubleValue };
            utilization_value += value;
        }
        Ok(utilization_value)
    }

    pub fn close(mut self) -> Result<()> {
        self.query_handle.close_query()
    }

    fn collect_query_data(&self) -> Result<()> {
        unsafe { PDH_FUNCTION(PdhCollectQueryData(self.query_handle.0)).ok() }
    }
}
