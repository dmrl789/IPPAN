import { Badge, Card } from '../components/UI';

export default function StakingValidator() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Staking & Validator Management</h1>
          <p className="text-sm text-gray-600">
            On-chain staking flows require the staking pallet and validator registry services. Hook them up to unlock
            delegation, undelegation, and validator configuration.
          </p>
        </div>
        <Badge variant="info">Integration Required</Badge>
      </div>

      <Card title="Staking">
        <p className="text-sm text-gray-700">
          The previous mock staking dashboards have been removed. Integrate the staking RPC endpoints to present real validator
          sets, delegations, and reward calculations.
        </p>
      </Card>
    </div>
  );
}
