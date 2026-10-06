//! Where `neoomsi` leads now (`::network::official::resolve`), and the server's status there.
fn main() {
    match ::network::official::resolve() {
        Ok(url) => {
            println!("neoomsi -> {url}");
            match ::network::ws::query(::network::official::ALIAS, false) {
                Ok(i) => println!(
                    "status: {} ({} / {} players)",
                    i.name, i.players, i.max_players
                ),
                Err(e) => println!("status: {e}"),
            }
        }
        Err(e) => println!("neoomsi: {e}"),
    }
}
