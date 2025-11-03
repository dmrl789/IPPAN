pub const BLOCK_REWARD: u64 = 1_0000_0000; // 1 IPN in micro-IPN

pub fn distribute_rewards(validators: &[String]) -> Vec<(String, u64)> {
    let share = BLOCK_REWARD / validators.len() as u64;
    validators.iter().map(|v| (v.clone(), share)).collect()
}
