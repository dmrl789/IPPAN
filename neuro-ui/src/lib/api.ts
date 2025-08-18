import axios from "axios";

const API = axios.create({
  baseURL: import.meta.env.VITE_API_BASE || "http://localhost:3000",
});

export type MetricKV = { key:string; value:string };
export type HashTimer = { us:number; round_id:number };

export type ModelAsset = {
  id: number[]; owner: number[]; arch_id:number; version:number;
  weights_hash: number[]; size_bytes:number;
  train_parent: number[] | null; train_config: number[];
  license_id:number; metrics: MetricKV[]; provenance: number[];
  created_at: HashTimer;
};

export type DatasetAsset = {
  id:number[]; owner:number[]; schema:string; shards:number[][];
  license_id:number; pii_flags:number; consents:string[];
  quality_scores: MetricKV[]; provenance:number[];
  created_at: HashTimer;
};

export type Sla = { max_latency_ms:number; region:string; price_cap_ipn:string|number };
export type InferenceJob = {
  id:number[]; model_ref:number[]; input_commit:number[];
  sla:Sla; privacy: "Open"|"TEE"|"Zk";
  bid_window_ms:number; max_price_ipn:string|number; escrow_ipn:string|number;
  created_at: HashTimer;
};

export type Bid = {
  job_id:number[]; executor_id:number[]; price_ipn:string|number; est_latency_ms:number; tee:boolean;
};

export type ProofPoI = {
  PoI: {
    job_id:number[]; model_ref:number[]; output_commit:number[];
    runtime_ms:number; attest:null|{blob:number[]}; zk:null|{bytes:number[]};
  }
};

export const postModel = (m: ModelAsset) => API.post<string>("/models", m).then(r=>r.data);
export const postDataset = (d: DatasetAsset) => API.post<string>("/datasets", d).then(r=>r.data);
export const postInference = (j: InferenceJob) => API.post<string>("/jobs", j).then(r=>r.data);
export const postBid = (b: Bid) => API.post<string>("/bids", b).then(r=>r.data);
export const getWinner = (jobHex: string) => API.get(`/jobs/${jobHex}/winner`).then(r=>r.data);
export const submitPoI = (p: ProofPoI) => API.post<boolean>("/proofs", p).then(r=>r.data);
export const getProof = (idHex: string) => API.get(`/proofs/${idHex}`).then(r=>r.data);
