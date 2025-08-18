import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Card, Button, Field, Input, Textarea } from "../components/UI";
import { ensureLen, hexToBytes } from "../lib/hex";
import { postDataset } from "../lib/api";

export default function DatasetsPage() {
  const [ownerHex, setOwner] = useState("");
  const [idHex, setId] = useState("");
  const [schema, setSchema] = useState("jsonl");
  const [shardsHex, setShardsHex] = useState("0x" + "00".repeat(32) + "\n0x" + "11".repeat(32));

  const { mutateAsync, isPending, data, error } = useMutation({
    mutationFn: async () => {
      const owner = ensureLen(hexToBytes(ownerHex), 32, "owner");
      const id = ensureLen(hexToBytes(idHex), 32, "id");

      const shards = shardsHex.split(/\s+/).filter(Boolean).map(h => ensureLen(hexToBytes(h), 32, "shard"));

      return postDataset({
        id, owner, schema,
        shards,
        license_id: 0, pii_flags: 0, consents: [],
        quality_scores: [],
        provenance: [],
        created_at: { us: 0, round_id: 0 }
      });
    }
  });

  return (
    <div className="max-w-2xl">
      <Card title="Register DatasetAsset">
        <div className="grid gap-3">
          <Field label="Owner (32-byte hex)">
            <Input value={ownerHex} onChange={e=>setOwner(e.target.value)} />
          </Field>
          <Field label="Dataset ID (32-byte hex)">
            <Input value={idHex} onChange={e=>setId(e.target.value)} />
          </Field>
          <Field label="Schema (free text)">
            <Input value={schema} onChange={e=>setSchema(e.target.value)} />
          </Field>
          <Field label="Shard IDs (one 32B hex per line)">
            <Textarea rows={5} value={shardsHex} onChange={e=>setShardsHex(e.target.value)} />
          </Field>

          <Button disabled={isPending} onClick={()=>mutateAsync()}>Submit</Button>
          {data && <div className="text-green-600">✔ Stored dataset id (hex): {data}</div>}
          {error && <div className="text-red-600">{(error as Error).message}</div>}
        </div>
      </Card>
    </div>
  );
}
