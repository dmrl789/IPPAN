import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Card, Button, Field, Input } from "../components/UI";
import { ensureLen, hexToBytes } from "../lib/hex";
import { postInference } from "../lib/api";

export default function InferencePage() {
  const [jobIdHex, setJobId] = useState("");
  const [modelIdHex, setModelId] = useState("");
  const [inputCommitHex, setInputCommit] = useState("");
  const [region, setRegion] = useState("eu-south");
  const [maxLatency, setMaxLatency] = useState(120);
  const [priceCap, setPriceCap] = useState(100000);
  const [privacy, setPrivacy] = useState<"Open"|"TEE"|"Zk">("Open");

  const { mutateAsync, isPending, data, error } = useMutation({
    mutationFn: async () => {
      const id = ensureLen(hexToBytes(jobIdHex), 32, "job_id");
      const model_ref = ensureLen(hexToBytes(modelIdHex), 32, "model_ref");
      const input_commit = ensureLen(hexToBytes(inputCommitHex), 32, "input_commit");
      return postInference({
        id, model_ref, input_commit,
        sla: { max_latency_ms: maxLatency, region, price_cap_ipn: priceCap },
        privacy,
        bid_window_ms: 100,
        max_price_ipn: priceCap,
        escrow_ipn: priceCap,
        created_at: { us: 0, round_id: 0 }
      });
    }
  });

  return (
    <div className="max-w-2xl">
      <Card title="Post Inference Job">
        <div className="grid gap-3">
          <Field label="Job ID (32B hex)">
            <Input value={jobIdHex} onChange={e=>setJobId(e.target.value)} placeholder="0x…64" />
          </Field>
          <Field label="Model ID (32B hex)">
            <Input value={modelIdHex} onChange={e=>setModelId(e.target.value)} />
          </Field>
          <Field label="Input Commit (32B hex)">
            <Input value={inputCommitHex} onChange={e=>setInputCommit(e.target.value)} />
          </Field>

          <div className="grid grid-cols-2 gap-3">
            <Field label="Region"><Input value={region} onChange={e=>setRegion(e.target.value)} /></Field>
            <Field label="Max Latency (ms)"><Input type="number" value={maxLatency} onChange={e=>setMaxLatency(parseInt(e.target.value||"0"))} /></Field>
            <Field label="Price Cap (IPN units)"><Input type="number" value={priceCap} onChange={e=>setPriceCap(parseInt(e.target.value||"0"))} /></Field>
            <Field label="Privacy">
              <select className="border rounded px-3 py-2 bg-transparent" value={privacy} onChange={e=>setPrivacy(e.target.value as any)}>
                <option>Open</option><option>TEE</option><option>Zk</option>
              </select>
            </Field>
          </div>

          <Button disabled={isPending} onClick={()=>mutateAsync()}>Submit</Button>
          {data && <div className="text-green-600">✔ Posted Job (server echoes hex id): {data}</div>}
          {error && <div className="text-red-600">{(error as Error).message}</div>}
        </div>
      </Card>
    </div>
  );
}
