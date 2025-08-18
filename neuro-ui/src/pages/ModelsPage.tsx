import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Card, Button, Field, Input } from "../components/UI";
import { ensureLen, hexToBytes } from "../lib/hex";
import { postModel } from "../lib/api";

export default function ModelsPage() {
  const [ownerHex, setOwner] = useState("");
  const [weightsHex, setWeights] = useState("");
  const [idHex, setId] = useState("");
  const [size, setSize] = useState<number>(0);

  const { mutateAsync, isPending, data, error } = useMutation({
    mutationFn: async () => {
      const owner = ensureLen(hexToBytes(ownerHex), 32, "owner");
      const weights_hash = ensureLen(hexToBytes(weightsHex), 32, "weights_hash");
      const id = ensureLen(hexToBytes(idHex), 32, "id");
      const train_config = new Array(32).fill(0);

      return postModel({
        id,
        owner,
        arch_id: 1,
        version: 1,
        weights_hash,
        size_bytes: size||0,
        train_parent: null,
        train_config,
        license_id: 0,
        metrics: [],
        provenance: [],
        created_at: { us: 0, round_id: 0 },
      });
    }
  });

  return (
    <div className="max-w-2xl">
      <Card title="Register ModelAsset">
        <div className="grid gap-3">
          <Field label="Owner (32-byte hex)">
            <Input value={ownerHex} onChange={e=>setOwner(e.target.value)} placeholder="0x…64 hex chars" />
          </Field>
          <Field label="Weights Hash (32-byte hex BLAKE3)">
            <Input value={weightsHex} onChange={e=>setWeights(e.target.value)} placeholder="0x…64" />
          </Field>
          <Field label="Model ID (32-byte hex)">
            <Input value={idHex} onChange={e=>setId(e.target.value)} placeholder="0x…64" />
          </Field>
          <Field label="Model Size (bytes)">
            <Input type="number" value={size} onChange={e=>setSize(parseInt(e.target.value||"0"))} />
          </Field>

          <Button disabled={isPending} onClick={()=>mutateAsync()}>Submit</Button>

          {data && <div className="text-green-600">✔ Stored model id (hex): {data}</div>}
          {error && <div className="text-red-600">{(error as Error).message}</div>}
          <p className="text-xs text-gray-500">
            Tip: For now, compute <em>Model ID = blake3(weights_hash)</em> offline and paste both as hex.
          </p>
        </div>
      </Card>
    </div>
  );
}
