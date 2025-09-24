import { Badge, Card } from '../components/UI';

export default function InferencePage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Inference Marketplace</h1>
          <p className="text-sm text-gray-600">
            Connect the inference orchestration service to submit jobs, monitor GPU queues, and inspect execution logs.
          </p>
        </div>
        <Badge variant="info">Integration Required</Badge>
      </div>

      <Card title="Inference Jobs">
        <p className="text-sm text-gray-700">
          All mock workflows have been removed. Deploy the inference microservice or smart contracts that manage inference
          auctions to enable this screen.
        </p>
      </Card>
    </div>
  );
}
