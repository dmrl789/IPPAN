import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Badge, Button, Card, Field, Input, LoadingSpinner } from '../../components/UI';
import { AccountRecord, fetchAccounts } from '../../lib/api';

const formatBalance = (balance: number) => balance.toLocaleString();

export default function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountRecord[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const cancelledRef = useRef(false);

  const loadAccounts = useCallback(
    async (initial: boolean) => {
      if (cancelledRef.current) {
        return;
      }

      initial ? setIsLoading(true) : setRefreshing(true);
      setError(null);

      try {
        const response = await fetchAccounts();
        if (!cancelledRef.current) {
          setAccounts(response);
        }
      } catch (err) {
        if (!cancelledRef.current) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelledRef.current) {
          initial ? setIsLoading(false) : setRefreshing(false);
        }
      }
    },
    [],
  );

  useEffect(() => {
    cancelledRef.current = false;
    loadAccounts(true);
    const interval = setInterval(() => loadAccounts(false), 20_000);

    return () => {
      cancelledRef.current = true;
      clearInterval(interval);
    };
  }, [loadAccounts]);

  const filteredAccounts = useMemo(() => {
    if (!search) {
      return accounts;
    }

    const needle = search.toLowerCase();
    return accounts.filter((account) =>
      [account.address, String(account.balance), String(account.nonce)]
        .filter(Boolean)
        .some((value) => value.toLowerCase().includes(needle)),
    );
  }, [accounts, search]);

  if (isLoading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <LoadingSpinner />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Accounts</h1>
          <p className="text-sm text-gray-600">
            All known L1 accounts discovered from the node&apos;s local state.
          </p>
        </div>
        <Badge variant="success">Live Network</Badge>
      </div>

      <Card title="Search & Refresh">
        <div className="grid gap-4 md:grid-cols-[1fr_auto]">
          <Field label="Search by address or amount">
            <Input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Enter account address or balance"
            />
          </Field>
          <div className="flex items-end justify-end">
            <Button onClick={() => loadAccounts(false)} disabled={refreshing}>
              {refreshing ? 'Refreshing…' : 'Refresh'}
            </Button>
          </div>
        </div>
      </Card>

      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 p-4 text-sm text-red-700">{error}</div>
      )}

      <Card title={`Accounts (${filteredAccounts.length})`}>
        {filteredAccounts.length === 0 ? (
          <div className="p-8 text-center text-gray-500">No accounts found.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="min-w-full text-left text-sm">
              <thead>
                <tr className="border-b bg-gray-50 text-xs uppercase tracking-wide text-gray-500">
                  <th className="px-4 py-3">Address</th>
                  <th className="px-4 py-3 text-right">Balance</th>
                  <th className="px-4 py-3 text-right">Nonce</th>
                </tr>
              </thead>
              <tbody>
                {filteredAccounts.map((account) => (
                  <tr key={account.address} className="border-b last:border-b-0">
                    <td className="px-4 py-3 font-mono text-xs text-gray-800">{account.address}</td>
                    <td className="px-4 py-3 text-right text-sm text-gray-700">
                      {formatBalance(account.balance)}
                    </td>
                    <td className="px-4 py-3 text-right text-sm text-gray-700">{account.nonce}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}
