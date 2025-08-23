// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use prometheus::{
    CounterVec, IntCounter, IntCounterVec, IntGauge, Registry, register_counter_vec_with_registry,
    register_int_counter_vec_with_registry, register_int_counter_with_registry,
    register_int_gauge_with_registry,
};
use std::sync::Arc;

fn int_gauge(registry: &Registry, name: &str, description: &str) -> IntGauge {
    register_int_gauge_with_registry!(name, description, registry)
        .expect(&format!("Failed to register IntGauge {name}"))
}

fn int_counter(registry: &Registry, name: &str, description: &str) -> IntCounter {
    register_int_counter_with_registry!(name, description, registry)
        .expect(&format!("Failed to register IntCounter {name}"))
}

fn int_counter_vec(
    registry: &Registry,
    name: &str,
    description: &str,
    labels: &[&str],
) -> IntCounterVec {
    register_int_counter_vec_with_registry!(name, description, labels, registry)
        .expect(&format!("Failed to register IntCounterVec {name}"))
}

fn counter_vec(registry: &Registry, name: &str, description: &str, labels: &[&str]) -> CounterVec {
    register_counter_vec_with_registry!(name, description, labels, registry)
        .expect(&format!("Failed to register CounterVec {name}"))
}

pub struct EpochMetrics {
    /// The current epoch ID. This is updated only when the AuthorityState finishes reconfiguration.
    pub current_epoch: IntGauge,

    /// Current voting right of the validator in the protocol. Updated at the start of epochs.
    pub current_voting_right: IntGauge,

    /// Total duration of the epoch. This is measured from when the current epoch store is opened,
    /// until the current epoch store is replaced with the next epoch store.
    pub epoch_total_duration: IntGauge,

    /// Number of checkpoints in the epoch.
    pub epoch_checkpoint_count: IntGauge,

    /// Number of transactions in the epoch.
    pub epoch_transaction_count: IntGauge,

    /// Total amount of gas rewards (i.e. computation gas cost) in the epoch.
    pub epoch_total_gas_reward: IntGauge,

    // An active validator reconfigures through the following steps:
    // 1. Halt validator (a.k.a. close epoch) and stop accepting user transaction certs.
    // 2. Finishes processing all pending certificates and then send EndOfPublish message.
    // 3. Stop accepting messages from consensus after seeing 2f+1 EndOfPublish messages.
    // 4. Creating the last checkpoint of the epoch by augmenting it with AdvanceEpoch transaction.
    // 5. CheckpointExecutor finishes executing the last checkpoint, and triggers reconfiguration.
    // 6. During reconfiguration, we tear down consensus, reconfigure state (at which point we opens
    //    up user certs), and start consensus again.
    // 7. After reconfiguration, and eventually consensus starts successfully, at some point the first
    //    checkpoint of the new epoch will be created.
    // We introduce various metrics to cover the latency of above steps.
    /// The duration from when the epoch is closed (i.e. validator halted) to when all pending
    /// certificates are processed (i.e. ready to send EndOfPublish message).
    /// This is the duration of (1) through (2) above.
    pub epoch_pending_certs_processed_time_since_epoch_close_ms: IntGauge,

    /// The interval from when the epoch is closed to when we receive 2f+1 EndOfPublish messages.
    /// This is the duration of (1) through (3) above.
    pub epoch_end_of_publish_quorum_time_since_epoch_close_ms: IntGauge,

    /// The interval from when the epoch is closed to when we created the last checkpoint of the
    /// epoch.
    /// This is the duration of (1) through (4) above.
    pub epoch_last_checkpoint_created_time_since_epoch_close_ms: IntGauge,

    /// The interval from when the epoch is closed to when we finished executing the last transaction
    /// of the checkpoint (and hence triggering reconfiguration process).
    /// This is the duration of (1) through (5) above.
    pub epoch_reconfig_start_time_since_epoch_close_ms: IntGauge,

    /// The total duration when this validator is halted, and hence does not accept certs from users.
    /// This is the duration of (1) through (6) above, and is the most important latency metric
    /// reflecting reconfiguration delay for each validator.
    pub epoch_validator_halt_duration_ms: IntGauge,

    /// The interval from when the epoch begins (i.e. right after state reconfigure, when the new
    /// epoch_store is created), to when the first checkpoint of the epoch is ready for creation locally.
    /// This is (7) above, and is a good proxy to how long it takes for the validator
    /// to become useful in the network after reconfiguration.
    // TODO: This needs to be reported properly.
    pub epoch_first_checkpoint_created_time_since_epoch_begin_ms: IntGauge,

    /// Whether we are running in safe mode where reward distribution and tokenomics are disabled.
    pub is_safe_mode: IntGauge,

    /// When building the last checkpoint of the epoch, we execute advance epoch transaction once
    /// without committing results to the store. It's useful to know whether this execution leads
    /// to safe_mode, since in theory the result could be different from checkpoint executor.
    pub checkpoint_builder_advance_epoch_is_safe_mode: IntGauge,

    /// Buffer stake current in effect for this epoch
    pub effective_buffer_stake: IntGauge,

    /// Set to 1 if the random beacon DKG protocol failed for the most recent epoch.
    pub epoch_random_beacon_dkg_failed: IntGauge,

    /// The number of shares held by this node after the random beacon DKG protocol completed.
    pub epoch_random_beacon_dkg_num_shares: IntGauge,

    /// The amount of time taken from epoch start to completion of random beacon DKG protocol,
    /// for the most recent epoch.
    pub epoch_random_beacon_dkg_epoch_start_completion_time_ms: IntGauge,

    /// The amount of time taken to complete random beacon DKG protocol from the time it was
    /// started (which may be a bit after the epcoh began), for the most recent epoch.
    pub epoch_random_beacon_dkg_completion_time_ms: IntGauge,

    /// The amount of time taken to start first phase of the random beacon DKG protocol,
    /// at which point the node has submitted a DKG Message, for the most recent epoch.
    pub epoch_random_beacon_dkg_message_time_ms: IntGauge,

    /// The amount of time taken to complete first phase of the random beacon DKG protocol,
    /// at which point the node has submitted a DKG Confirmation, for the most recent epoch.
    pub epoch_random_beacon_dkg_confirmation_time_ms: IntGauge,

    /// The number of execution time observations messages shared by this node.
    pub epoch_execution_time_observations_shared: IntCounter,

    /// The number of execution time observations messages intended to be shared by this node, annotated with reason.
    pub epoch_execution_time_observations_sharing_reason: IntCounterVec,

    /// The number of execution time measurements dropped due to backpressure from the observer.
    pub epoch_execution_time_measurements_dropped: IntCounter,

    /// The number of execution time consensus messages dropped.
    pub epoch_execution_time_observations_dropped: IntCounterVec,

    /// The number of cached indebted objects in the execution time observer.
    pub epoch_execution_time_observer_indebted_objects: IntGauge,

    /// The number of objects tracked by the object utilization cache.
    pub epoch_execution_time_observer_utilization_cache_size: IntGauge,

    /// The number of objects determined by the execution time observer to be overutilized.
    /// Note: this may overcount if objects are evicted from the cache before being computed
    /// as not-overutilized.
    pub epoch_execution_time_observer_overutilized_objects: IntGauge,

    /// Per-object utilization for objects that were overutilized at least once at some
    /// point in their lifetime.
    /// Note: This metric is disabled by default as it may have very large cardinality.
    pub epoch_execution_time_observer_object_utilization: CounterVec,

    /// The number of execution time observations loaded at start of epoch.
    pub epoch_execution_time_observations_loaded: IntGauge,

    /// The number of consensus output items in the quarantine.
    pub consensus_quarantine_queue_size: IntGauge,

    /// The number of shared object assignments in the quarantine.
    pub shared_object_assignments_size: IntGauge,
}

impl EpochMetrics {
    pub fn new(registry: &Registry) -> Arc<Self> {
        let this = Self {
            current_epoch: int_gauge(registry, "current_epoch", "Current epoch ID"),
            current_voting_right: int_gauge(
                registry,
                "current_voting_right",
                "Current voting right of the validator",
            ),
            epoch_checkpoint_count: int_gauge(
                registry,
                "epoch_checkpoint_count",
                "Number of checkpoints in the epoch",
            ),
            epoch_total_duration: int_gauge(
                registry,
                "epoch_total_duration",
                "Total duration of the epoch",
            ),
            epoch_transaction_count: int_gauge(
                registry,
                "epoch_transaction_count",
                "Number of transactions in the epoch",
            ),
            epoch_total_gas_reward: int_gauge(
                registry,
                "epoch_total_gas_reward",
                "Total amount of gas rewards (i.e. computation gas cost) in the epoch",
            ),
            epoch_pending_certs_processed_time_since_epoch_close_ms: int_gauge(
                registry,
                "epoch_pending_certs_processed_time_since_epoch_close_ms",
                "Time interval from when epoch was closed to when all pending certificates are processed",
            ),
            epoch_end_of_publish_quorum_time_since_epoch_close_ms: int_gauge(
                registry,
                "epoch_end_of_publish_quorum_time_since_epoch_close_ms",
                "Time interval from when epoch was closed to when 2f+1 EndOfPublish messages are received",
            ),
            epoch_last_checkpoint_created_time_since_epoch_close_ms: int_gauge(
                registry,
                "epoch_last_checkpoint_created_time_since_epoch_close_ms",
                "Time interval from when epoch was closed to when the last checkpoint of the epoch is created",
            ),
            epoch_reconfig_start_time_since_epoch_close_ms: int_gauge(
                registry,
                "epoch_reconfig_start_time_since_epoch_close_ms",
                "Total time duration from when epoch was closed to when we begin to reconfigure the validator",
            ),
            epoch_validator_halt_duration_ms: int_gauge(
                registry,
                "epoch_validator_halt_duration_ms",
                "Total time duration when the validator was halted (i.e. epoch closed)",
            ),
            epoch_first_checkpoint_created_time_since_epoch_begin_ms: int_gauge(
                registry,
                "epoch_first_checkpoint_created_time_since_epoch_begin_ms",
                "Time interval from when the epoch opens at new epoch to the first checkpoint is created locally",
            ),
            is_safe_mode: int_gauge(
                registry,
                "is_safe_mode",
                "Whether we are running in safe mode",
            ),
            checkpoint_builder_advance_epoch_is_safe_mode: int_gauge(
                registry,
                "checkpoint_builder_advance_epoch_is_safe_mode",
                "Whether the advance epoch execution leads to safe mode while building the last checkpoint",
            ),
            effective_buffer_stake: int_gauge(
                registry,
                "effective_buffer_stake",
                "Buffer stake current in effect for this epoch",
            ),
            epoch_random_beacon_dkg_failed: int_gauge(
                registry,
                "epoch_random_beacon_dkg_failed",
                "Set to 1 if the random beacon DKG protocol failed for the most recent epoch.",
            ),
            epoch_random_beacon_dkg_num_shares: int_gauge(
                registry,
                "epoch_random_beacon_dkg_num_shares",
                "The number of shares held by this node after the random beacon DKG protocol completed",
            ),
            epoch_random_beacon_dkg_epoch_start_completion_time_ms: int_gauge(
                registry,
                "epoch_random_beacon_dkg_epoch_start_completion_time_ms",
                "The amount of time taken from epoch start to completion of random beacon DKG protocol, for the most recent epoch",
            ),
            epoch_random_beacon_dkg_completion_time_ms: int_gauge(
                registry,
                "epoch_random_beacon_dkg_completion_time_ms",
                "The amount of time taken to complete random beacon DKG protocol from the time it was started (which may be a bit after the epoch began), for the most recent epoch",
            ),
            epoch_random_beacon_dkg_message_time_ms: int_gauge(
                registry,
                "epoch_random_beacon_dkg_message_time_ms",
                "The amount of time taken to start first phase of the random beacon DKG protocol, at which point the node has submitted a DKG Message, for the most recent epoch",
            ),
            epoch_random_beacon_dkg_confirmation_time_ms: int_gauge(
                registry,
                "epoch_random_beacon_dkg_confirmation_time_ms",
                "The amount of time taken to complete first phase of the random beacon DKG protocol, at which point the node has submitted a DKG Confirmation, for the most recent epoch",
            ),
            epoch_execution_time_observations_shared: int_counter(
                registry,
                "epoch_execution_time_observations_shared",
                "The number of execution time observations messages shared by this node",
            ),
            epoch_execution_time_observations_sharing_reason: int_counter_vec(
                registry,
                "epoch_execution_time_observations_sharing_reason",
                "The number of execution time observations messages intended to be shared by this node, annotated with reason",
                &["reason"],
            ),
            epoch_execution_time_measurements_dropped: int_counter(
                registry,
                "epoch_execution_time_measurements_dropped",
                "The number of execution time measurements dropped due to backpressure from the observer",
            ),
            epoch_execution_time_observations_dropped: int_counter_vec(
                registry,
                "epoch_execution_time_observations_dropped",
                "The number of execution time observations dropped",
                &["reason"],
            ),
            epoch_execution_time_observer_indebted_objects: int_gauge(
                registry,
                "epoch_execution_time_observer_indebted_objects",
                "The number of cached indebted objects in the execution time observer",
            ),
            epoch_execution_time_observer_utilization_cache_size: int_gauge(
                registry,
                "epoch_execution_time_observer_utilization_cache_size",
                "The number of objects tracked by the object utilization cache",
            ),
            epoch_execution_time_observer_overutilized_objects: int_gauge(
                registry,
                "epoch_execution_time_observer_overutilized_objects",
                "The number of objects determined by the execution time observer to be overutilized. Note: this may overcount if objects are evicted from the cache before being computed as not-overutilized.",
            ),
            epoch_execution_time_observer_object_utilization: counter_vec(
                registry,
                "epoch_execution_time_observer_object_utilization",
                "Per-object utilization for objects that were overutilized at least once at some point in their lifetime",
                &["object_id"],
            ),
            epoch_execution_time_observations_loaded: int_gauge(
                registry,
                "epoch_execution_time_observations_loaded",
                "The number of execution time observations loaded at start of epoch",
            ),
            consensus_quarantine_queue_size: int_gauge(
                registry,
                "consensus_quarantine_queue_size",
                "The number of consensus output items in the quarantine",
            ),
            shared_object_assignments_size: int_gauge(
                registry,
                "shared_object_assignments_size",
                "The number of shared object assignments in the quarantine",
            ),
        };
        Arc::new(this)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::Registry;
    use std::panic;

    #[test]
    fn register_int_gauge_failure_message() {
        let registry = Registry::new();
        let _ = int_gauge(&registry, "test_metric", "desc");
        let result = panic::catch_unwind(|| {
            let _ = int_gauge(&registry, "test_metric", "desc");
        });
        let err = result.expect_err("expected panic");
        let msg = err
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| err.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(msg.contains("Failed to register IntGauge test_metric"));
    }
}
