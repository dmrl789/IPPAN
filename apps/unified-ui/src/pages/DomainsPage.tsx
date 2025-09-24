import { Card, Badge } from '../components/UI';

export default function DomainsPage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Domain Management</h1>
          <p className="text-sm text-gray-600">
            The on-chain domain registry requires a dedicated service. Connect a registry module to manage
            blockchain domains and DNS records from this interface.
          </p>
        </div>
        <Badge variant="info">Integration Required</Badge>
      </div>

      <Card title="Domain Registry">
        <div className="space-y-3 text-sm text-gray-700">
          <p>
            The demo placeholders have been removed. Configure the IPPAN domain registry service and update the API
            handlers to enable domain search, registration, renewal, and DNS management features.
          </p>
          <p>
            Once the registry API is available, this page will automatically surface live data. For now no actions are
            performed client-side.
          </p>
        </div>
      </Card>
    </div>
  );
}
