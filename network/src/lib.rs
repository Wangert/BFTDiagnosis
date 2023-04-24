pub mod peer;
pub mod transport;
// pub mod discovery;
// pub mod gossipsub;
pub mod p2p_protocols;
pub mod base_swarm;
// pub mod tcp;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
