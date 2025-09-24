import { Badge, Card } from '../components/UI';

export default function InteroperabilityPage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Interoperability</h1>
          <p className="text-sm text-gray-600">
            Bridge monitoring and exit processing rely on external rollup connectors. Provide a live L2 service to display
            commitments and exit queues.
          </p>
        </div>
        <Badge variant="info">Integration Required</Badge>
      </div>

      <Card title="L2 Connectivity">
        <p className="text-sm text-gray-700">
          Placeholder data for rollup commits and exit queues has been removed. Attach the interoperability daemon to make
          this dashboard operational.
        </p>
      </Card>
    </div>
  );
}
