import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Card, Button, Field, Input } from "../components/UI";
import { ensureLen, hexToBytes, strip0x } from "../lib/hex";
import { submitPoI, getProof } from "../lib/api";

export default function ProofsPage() {
  const [jobIdHex, setJob] = useState("");
  const [modelIdHex, setModel] = useState("");
  const [outputCommitHex, setOut] = useState("");
  const [runtimeMs, setRuntime] = useState(47);
  const [fetched, setFetched] = useState<any>(null);

  const submit = useMutation({
    mutationFn: async () => {
      const job_id = ensureLen(hexToBytes(jobIdHex), 32, "job_id");
      const model_ref = ensureLen(hexToBytes(modelIdHex), 32, "model_ref");
      const output_commit = ensureLen(hexToBytes(outputCommitHex), 32, "output_commit");
      return submitPoI({ PoI: { job_id, model_ref, output_commit, runtime_ms: runtimeMs, attest: null, zk: null }});
    }
  });

  const fetch = async () => {
    const res = await getProof(strip0x(jobIdHex));
    setFetched(res);
  };

  return (
    <div className="max-w-2xl space-y-6">
      <Card title="Submit Proof-of-Inference (PoI)">
        <div className="grid gap-3">
          <Field label="Job ID (32B hex)"><Input value={jobIdHex} onChange={e=>setJob(e.target.value)} /></Field>
          <Field label="Model ID (32B hex)"><Input value={modelIdHex} onChange={e=>setModel(e.target.value)} /></Field>
          <Field label="Output Commit (32B hex)"><Input value={outputCommitHex} onChange={e=>setOut(e.target.value)} /></Field>
          <Field label="Runtime (ms)"><Input type="number" value={runtimeMs} onChange={e=>setRuntime(parseInt(e.target.value||"0"))} /></Field>
          <Button onClick={()=>submit.mutate()}>Submit PoI</Button>
          {submit.data && <div className="text-green-600">✔ Proof stored</div>}
          {submit.error && <div className="text-red-600">{(submit.error as Error).message}</div>}
        </div>
      </Card>

      <Card title="Fetch Proof by Job ID">
        <div className="grid gap-3">
          <Field label="Job ID (32B hex)"><Input value={jobIdHex} onChange={e=>setJob(e.target.value)} /></Field>
          <Button onClick={fetch}>Get Proof</Button>
          {fetched && <pre className="text-xs p-3 bg-black/5 rounded overflow-auto">{JSON.stringify(fetched, null, 2)}</pre>}
        </div>
      </Card>
    </div>
  );
}
