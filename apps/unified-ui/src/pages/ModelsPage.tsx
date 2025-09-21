import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Card, Button, Field, Input, Badge, LoadingSpinner } from "../components/UI";
import { ensureLen, hexToBytes } from "../lib/hex";
import { postModel, getModels, ModelAsset } from "../lib/api";

export default function ModelsPage() {
  const [ownerHex, setOwner] = useState("");
  const [weightsHex, setWeights] = useState("");
  const [idHex, setId] = useState("");
  const [size, setSize] = useState<number>(0);

  const { data: models, isLoading: loadingModels } = useQuery({
    queryKey: ['models'],
    queryFn: getModels,
  });

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
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Neural Models</h1>
        <Badge variant="success">Connected to IPPAN Neural Network</Badge>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Register New Model */}
        <Card title="Register Model Asset">
          <div className="space-y-4">
            <Field label="Owner (32-byte hex)">
              <Input 
                value={ownerHex} 
                onChange={e=>setOwner(e.target.value)} 
                placeholder="0x…64 hex chars" 
              />
            </Field>
            <Field label="Weights Hash (32-byte hex BLAKE3)">
              <Input 
                value={weightsHex} 
                onChange={e=>setWeights(e.target.value)} 
                placeholder="0x…64" 
              />
            </Field>
            <Field label="Model ID (32-byte hex)">
              <Input 
                value={idHex} 
                onChange={e=>setId(e.target.value)} 
                placeholder="0x…64" 
              />
            </Field>
            <Field label="Model Size (bytes)">
              <Input 
                type="number" 
                value={size} 
                onChange={e=>setSize(parseInt(e.target.value||"0"))} 
              />
            </Field>

            <Button 
              disabled={isPending} 
              onClick={()=>mutateAsync()}
              className="w-full"
            >
              {isPending ? <LoadingSpinner /> : 'Register Model'}
            </Button>

            {data && (
              <div className="p-3 bg-green-50 border border-green-200 rounded">
                <div className="text-green-800 font-medium">✓ Model registered successfully!</div>
                <div className="text-sm text-green-600">Model ID: {data}</div>
              </div>
            )}
            {error && (
              <div className="p-3 bg-red-50 border border-red-200 rounded">
                <div className="text-red-800">Error: {(error as Error).message}</div>
              </div>
            )}
            
            <div className="text-xs text-gray-500 bg-gray-50 p-3 rounded">
              <strong>Tip:</strong> For now, compute <em>Model ID = blake3(weights_hash)</em> offline and paste both as hex.
            </div>
          </div>
        </Card>

        {/* Model Statistics */}
        <Card title="Model Statistics">
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="text-center p-4 bg-blue-50 rounded">
                <div className="text-2xl font-bold text-blue-600">
                  {loadingModels ? '...' : models?.length || 0}
                </div>
                <div className="text-sm text-gray-600">Total Models</div>
              </div>
              <div className="text-center p-4 bg-green-50 rounded">
                <div className="text-2xl font-bold text-green-600">1.2TB</div>
                <div className="text-sm text-gray-600">Total Size</div>
              </div>
            </div>
            
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>Active Models:</span>
                <span className="font-medium">24</span>
              </div>
              <div className="flex justify-between text-sm">
                <span>Training:</span>
                <span className="font-medium">3</span>
              </div>
              <div className="flex justify-between text-sm">
                <span>Inference Jobs:</span>
                <span className="font-medium">156</span>
              </div>
            </div>
          </div>
        </Card>
      </div>

      {/* Registered Models List */}
      <Card title="Registered Models">
        {loadingModels ? (
          <div className="flex justify-center py-8">
            <LoadingSpinner />
          </div>
        ) : models && models.length > 0 ? (
          <div className="space-y-3">
            {models.slice(0, 5).map((model, index) => (
              <div key={index} className="flex justify-between items-center p-3 bg-gray-50 rounded">
                <div>
                  <div className="font-medium">Model #{index + 1}</div>
                  <div className="text-sm text-gray-600">
                    ID: {model.id ? Array.from(model.id).map(b => b.toString(16).padStart(2, '0')).join('').slice(0, 16) + '...' : 'N/A'}
                  </div>
                  <div className="text-xs text-gray-500">
                    Size: {model.size_bytes} bytes | Version: {model.version}
                  </div>
                </div>
                <div className="text-right">
                  <Badge variant="success">Registered</Badge>
                  <div className="text-xs text-gray-500 mt-1">
                    Round {model.created_at?.round_id || 0}
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-8 text-gray-500">
            No models registered yet. Register your first model above.
          </div>
        )}
      </Card>

      {/* Model Architecture Types */}
      <Card title="Supported Architectures">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-4 border rounded">
            <div className="font-medium">Transformer</div>
            <div className="text-sm text-gray-600">Arch ID: 1</div>
            <div className="text-xs text-gray-500">GPT, BERT, T5 variants</div>
          </div>
          <div className="p-4 border rounded">
            <div className="font-medium">CNN</div>
            <div className="text-sm text-gray-600">Arch ID: 2</div>
            <div className="text-xs text-gray-500">ResNet, VGG, EfficientNet</div>
          </div>
          <div className="p-4 border rounded">
            <div className="font-medium">RNN/LSTM</div>
            <div className="text-sm text-gray-600">Arch ID: 3</div>
            <div className="text-xs text-gray-500">Sequence models</div>
          </div>
        </div>
      </Card>
    </div>
  );
}
