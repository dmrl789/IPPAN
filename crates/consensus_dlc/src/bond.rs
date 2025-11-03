pub const VALIDATOR_BOND: u64 = 10 * 10u64.pow(8); // 10 IPN

#[derive(Clone, Debug)]
pub struct ValidatorBond {
    pub owner: String,
    pub amount: u64,
    pub active: bool,
}

impl ValidatorBond {
    pub fn new(owner: impl Into<String>) -> Self {
        Self { owner: owner.into(), amount: VALIDATOR_BOND, active: true }
    }

    pub fn slash(&mut self) {
        self.amount = 0;
        self.active = false;
    }
}
