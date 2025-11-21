// Copyright 2025 Rigatoni Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Tests for metrics instrumentation module.

use rigatoni_core::metrics::{
    PipelineStatus, Timer, BATCH_DURATION_SECONDS, EVENTS_PROCESSED_TOTAL, METRIC_PREFIX,
};

#[test]
fn test_metric_names() {
    // Ensure all metric names follow conventions
    assert!(EVENTS_PROCESSED_TOTAL.starts_with(METRIC_PREFIX));
    assert!(EVENTS_PROCESSED_TOTAL.ends_with("_total"));
    assert!(BATCH_DURATION_SECONDS.ends_with("_seconds"));
}

#[test]
fn test_pipeline_status_values() {
    assert_eq!(PipelineStatus::Stopped as u8, 0);
    assert_eq!(PipelineStatus::Running as u8, 1);
    assert_eq!(PipelineStatus::Error as u8, 2);
}

#[test]
fn test_timer() {
    use std::thread;
    use std::time::Duration;

    let recorded_duration = std::sync::Arc::new(std::sync::Mutex::new(None));
    let recorded_duration_clone = recorded_duration.clone();

    {
        let _timer = Timer::new("test", move |duration, _label| {
            *recorded_duration_clone.lock().unwrap() = Some(duration);
        });
        thread::sleep(Duration::from_millis(10));
    }

    let duration = recorded_duration.lock().unwrap().unwrap();
    assert!(duration.as_millis() >= 10);
}
