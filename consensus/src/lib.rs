// pub mod pbft;
// pub mod basic_hotstuff;
pub mod chain_hotstuff;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
