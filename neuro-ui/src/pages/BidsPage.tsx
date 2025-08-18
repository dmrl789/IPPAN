import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Card, Button, Field, Input } from "../components/UI";
import { ensureLen, hexToBytes, strip0x } from "../lib/hex";
import { postBid, getWinner } from "../lib/api";

export default function BidsPage() {
  const [jobIdHex, setJob] = useState("");
  const [execHex, setExec] = useState("");
  const [price, setPrice] = useState(80000);
  const [latency, setLatency] = useState(115);
  const [tee, setTee] = useState(false);
  const [winner, setWinner] = useState<any>(null);

  const placeBid = useMutation({
    mutationFn: async () => {
      const job_id = ensureLen(hexToBytes(jobIdHex), 32, "job_id");
      const executor_id = ensureLen(hexToBytes(execHex), 32, "executor_id");
      return postBid({ job_id, executor_id, price_ipn: price, est_latency_ms: latency, tee });
    }
  });

  const fetchWinner = async () => {
    const hex = strip0x(jobIdHex);
    const res = await getWinner(hex);
    setWinner(res);
  };

  return (
    <div className="max-w-2xl space-y-6">
      <Card title="Place Bid">
        <div className="grid gap-3">
          <Field label="Job ID (32B hex)"><Input value={jobIdHex} onChange={e=>setJob(e.target.value)} /></Field>
          <Field label="Executor ID (32B hex)"><Input value={execHex} onChange={e=>setExec(e.target.value)} /></Field>
          <div className="grid grid-cols-3 gap-3">
            <Field label="Price (IPN)"><Input type="number" value={price} onChange={e=>setPrice(parseInt(e.target.value||"0"))} /></Field>
            <Field label="Est. Latency (ms)"><Input type="number" value={latency} onChange={e=>setLatency(parseInt(e.target.value||"0"))} /></Field>
            <Field label="TEE"><Input type="checkbox" checked={tee} onChange={e=>setTee(e.target.checked)} /></Field>
          </div>
          <Button onClick={()=>placeBid.mutate()}>Submit Bid</Button>
          {placeBid.data && <div className="text-green-600">✔ Bid accepted</div>}
          {placeBid.error && <div className="text-red-600">{(placeBid.error as Error).message}</div>}
        </div>
      </Card>

      <Card title="Winner">
        <div className="grid gap-3">
          <Field label="Job ID (32B hex)"><Input value={jobIdHex} onChange={e=>setJob(e.target.value)} /></Field>
          <div className="flex gap-2">
            <Button onClick={fetchWinner}>Get Winner</Button>
          </div>
          {winner && (
            <pre className="text-xs p-3 bg-black/5 rounded overflow-auto">{JSON.stringify(winner, null, 2)}</pre>
          )}
        </div>
      </Card>
    </div>
  );
}
