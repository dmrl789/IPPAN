pub struct ChunkAccessStat {
    pub chunk_id: String,
    pub last_access: HashTimer,
    pub region: String,
    pub score: f32,
}

impl ChunkAccessStat {
    pub fn new(chunk_id: String, last_access: HashTimer, region: String, score: f32) -> Self {
        ChunkAccessStat {
            chunk_id,
            last_access,
            region,
            score,
        }
    }
} 