use crate::tx_engine::ClientSnapshot;

pub fn print_clients_snapshot(snapshots: &[ClientSnapshot]) {
    println!("client,available,held,total,locked");
    for snapshot in snapshots {
        println!(
            "{},{:.4},{:.4},{:.4},{}",
            snapshot.client_id,
            snapshot.available.inner(),
            snapshot.held.inner(),
            snapshot.total().inner(),
            snapshot.locked
        );
    }
}
