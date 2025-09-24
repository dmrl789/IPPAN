import React, { useEffect, useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View
} from 'react-native';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

import { Card } from '../../components/Card';
import { Button } from '../../components/Button';
import { InfoRow } from '../../components/InfoRow';
import { Tag } from '../../components/Tag';
import { useApi } from '../../providers/ApiProvider';
import { useWallet } from '../../providers/WalletProvider';
import {
  fetchWalletBalance,
  fetchWalletTransactions,
  submitPayment,
  fetchDomains,
  type WalletTransaction,
  type DomainRecord,
  type SubmitPaymentInput,
  type SubmitPaymentResponse
} from '../../api';
import {
  mockDomainUpdates,
  mockDnsUpdates,
  mockTldUpdates
} from '../../data/domainUpdates';
import {
  formatDateTime,
  formatNumber,
  formatPercent,
  truncateAddress
} from '../../utils/format';

const STORAGE_SAMPLE = {
  reservedGb: 50,
  usedGb: 22.5,
  replication: 3,
  uptimePct: 99.9,
  bandwidthUsedGb: 48,
  pinnedCount: 87
};

const AVAILABILITY_SAMPLE = [
  { node: 'validator-eu-1', availability: 99.92, region: 'Frankfurt' },
  { node: 'validator-us-1', availability: 99.87, region: 'Ashburn' },
  { node: 'gateway-apac', availability: 99.71, region: 'Singapore' }
];

const ADDRESS_PLACEHOLDER = 'i0000000000000000000000000000000000000000000000000000000000000000';

export default function WalletOverviewScreen() {
  const { client } = useApi();
  const queryClient = useQueryClient();
  const {
    walletAddress,
    walletType,
    setWatchAddress,
    connectLocalWallet,
    disconnectWallet,
    signMessage
  } = useWallet();

  const [addressInput, setAddressInput] = useState(walletAddress ?? '');
  const [recipient, setRecipient] = useState('');
  const [amount, setAmount] = useState('');
  const [memo, setMemo] = useState('');
  const [isSettingAddress, setIsSettingAddress] = useState(false);

  useEffect(() => {
    setAddressInput(walletAddress ?? '');
  }, [walletAddress]);

  const walletEnabled = Boolean(walletAddress);

  const balanceQuery = useQuery({
    queryKey: ['wallet-balance', walletAddress],
    queryFn: () => fetchWalletBalance(client, walletAddress as string),
    enabled: walletEnabled,
    staleTime: 30_000
  });

  const transactionsQuery = useQuery({
    queryKey: ['wallet-transactions', walletAddress],
    queryFn: () => fetchWalletTransactions(client, walletAddress as string),
    enabled: walletEnabled,
    staleTime: 30_000
  });

  const domainsQuery = useQuery({
    queryKey: ['domains'],
    queryFn: () => fetchDomains(client),
    staleTime: 60_000
  });

  const paymentMutation = useMutation<SubmitPaymentResponse, Error, SubmitPaymentInput>({
    mutationFn: (payload) => submitPayment(client, payload),
    onSuccess: (result) => {
      if (result.success) {
        Alert.alert('Transaction submitted', result.txId ? `Tx hash: ${result.txId}` : 'The network accepted your transaction.');
        setAmount('');
        setRecipient('');
        setMemo('');
        queryClient.invalidateQueries({ queryKey: ['wallet-balance'] });
        queryClient.invalidateQueries({ queryKey: ['wallet-transactions'] });
      } else {
        Alert.alert('Unable to submit transaction', result.message || 'Unknown error');
      }
    },
    onError: (error: unknown) => {
      const message = error instanceof Error ? error.message : 'Unknown error';
      Alert.alert('Transaction failed', message);
    }
  });

  const assets = useMemo(() => {
    if (!balanceQuery.data) {
      return [] as { symbol: string; name: string; balance: number }[];
    }
    return [
      { symbol: 'IPN', name: 'IPPAN', balance: balanceQuery.data.balance },
      { symbol: 'STAKE', name: 'Staked IPN', balance: balanceQuery.data.staked }
    ];
  }, [balanceQuery.data]);

  const activities: WalletTransaction[] = useMemo(() => {
    return (transactionsQuery.data ?? []).slice(0, 6);
  }, [transactionsQuery.data]);

  const handleSetWatchAddress = async () => {
    try {
      setIsSettingAddress(true);
      await setWatchAddress(addressInput);
      Alert.alert('Wallet updated', 'Now tracking the provided address.');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unable to set address';
      Alert.alert('Invalid address', message);
    } finally {
      setIsSettingAddress(false);
    }
  };

  const handleConnectLocal = async () => {
    try {
      const addr = await connectLocalWallet();
      Alert.alert('Local wallet ready', `Generated address ${truncateAddress(addr, 10)}.`);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unable to create wallet';
      Alert.alert('Wallet error', message);
    }
  };

  const handleDisconnect = async () => {
    await disconnectWallet();
    Alert.alert('Wallet disconnected', 'This device is no longer linked to an IPPAN wallet.');
  };

  const handleSendPayment = async () => {
    if (!walletAddress) {
      Alert.alert('No wallet', 'Connect or watch a wallet before sending payments.');
      return;
    }
    const value = Number(amount);
    if (!Number.isFinite(value) || value <= 0) {
      Alert.alert('Invalid amount', 'Enter the amount to send in IPN.');
      return;
    }
    if (!recipient || recipient.length < 10) {
      Alert.alert('Recipient required', 'Enter a valid destination address.');
      return;
    }

    const fee = Math.max(0.01, Number((value * 0.002).toFixed(4)));
    const nonce = (balanceQuery.data?.nonce ?? 0) + 1;

    try {
      const signature = await signMessage(`${walletAddress}:${recipient}:${value}:${nonce}:${memo}`);
      paymentMutation.mutate({
        from: walletAddress,
        to: recipient,
        amount: value,
        fee,
        nonce,
        memo: memo || undefined,
        signature
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'This wallet cannot sign messages.';
      Alert.alert('Cannot sign transaction', message);
    }
  };

  return (
    <ScrollView style={styles.container} contentContainerStyle={styles.content}>
      <Text style={styles.heading}>IPPAN Wallet & Finance</Text>
      <Text style={styles.subheading}>Manage balances, payments, domains and storage from a single mobile dashboard.</Text>

      <Card
        title="Wallet status"
        subtitle="Link a watch-only address or promote this device to a local signer."
        action={<Tag label={walletEnabled ? 'Connected' : 'Not connected'} tone={walletEnabled ? 'success' : 'warning'} />}
      >
        <InfoRow label="Wallet mode" value={walletType === 'local' ? 'Local signer' : 'Watch only'} />
        <InfoRow
          label="Address"
          value={
            <Text style={styles.addressText} selectable>
              {walletAddress ? walletAddress : '—'}
            </Text>
          }
          align="top"
        />

        <View style={styles.inputRow}>
          <TextInput
            value={addressInput}
            onChangeText={setAddressInput}
            placeholder={ADDRESS_PLACEHOLDER}
            placeholderTextColor="#475569"
            style={styles.input}
            autoCapitalize="none"
            autoCorrect={false}
          />
        </View>

        <View style={styles.buttonRow}>
          <Button
            label="Set watch address"
            onPress={handleSetWatchAddress}
            disabled={isSettingAddress || !addressInput}
          />
          <Button label="Create local" onPress={handleConnectLocal} variant="secondary" />
          <Button label="Disconnect" onPress={handleDisconnect} variant="ghost" disabled={!walletEnabled} />
        </View>
      </Card>

      <Card title="Balances" subtitle="Realtime balances retrieved from the connected IPPAN node.">
        {balanceQuery.isFetching ? (
          <ActivityIndicator color="#38bdf8" />
        ) : (
          <View style={styles.assetList}>
            {assets.map((asset) => (
              <View key={asset.symbol} style={styles.assetItem}>
                <View>
                  <Text style={styles.assetSymbol}>{asset.symbol}</Text>
                  <Text style={styles.assetName}>{asset.name}</Text>
                </View>
                <Text style={styles.assetBalance}>{formatNumber(asset.balance)}</Text>
              </View>
            ))}
            {assets.length === 0 ? <Text style={styles.placeholder}>Connect a wallet to view balances.</Text> : null}
          </View>
        )}
        <InfoRow label="Nonce" value={balanceQuery.data?.nonce ?? '—'} />
        <InfoRow label="Pending" value={balanceQuery.data?.pendingTransactions?.length ?? 0} />
      </Card>

      <Card title="Payments & machine-to-machine settlement">
        <Text style={styles.label}>Recipient address</Text>
        <TextInput
          value={recipient}
          onChangeText={setRecipient}
          placeholder="i..."
          placeholderTextColor="#475569"
          style={styles.input}
          autoCapitalize="none"
          autoCorrect={false}
        />
        <Text style={styles.label}>Amount (IPN)</Text>
        <TextInput
          value={amount}
          onChangeText={setAmount}
          placeholder="10.5"
          placeholderTextColor="#475569"
          keyboardType="decimal-pad"
          style={styles.input}
        />
        <Text style={styles.label}>Memo (optional)</Text>
        <TextInput
          value={memo}
          onChangeText={setMemo}
          placeholder="Payment description"
          placeholderTextColor="#475569"
          style={[styles.input, styles.textArea]}
          multiline
          numberOfLines={3}
        />
        <Button
          label={paymentMutation.isPending ? 'Submitting…' : 'Submit payment'}
          onPress={handleSendPayment}
          disabled={paymentMutation.isPending}
        />
      </Card>

      <Card title="Recent activity" subtitle="Latest confirmed transactions for this wallet.">
        {transactionsQuery.isFetching ? (
          <ActivityIndicator color="#38bdf8" />
        ) : activities.length === 0 ? (
          <Text style={styles.placeholder}>No transactions yet.</Text>
        ) : (
          activities.map((activity) => (
            <View key={activity.id} style={styles.transactionItem}>
              <View style={{ flex: 1 }}>
                <Text style={styles.transactionTitle}>{activity.type ?? 'Transfer'}</Text>
                <Text style={styles.transactionSubtitle}>To {truncateAddress(activity.to)}</Text>
                <Text style={styles.transactionMeta}>{formatDateTime(activity.timestamp)}</Text>
              </View>
              <View style={{ alignItems: 'flex-end' }}>
                <Text style={styles.transactionAmount}>{formatNumber(activity.amount)}</Text>
                {activity.fee ? <Text style={styles.transactionFee}>Fee {formatNumber(activity.fee)}</Text> : null}
              </View>
            </View>
          ))
        )}
      </Card>

      <Card title="Domain management" subtitle="Track owned namespaces and recent registry updates.">
        {domainsQuery.isFetching ? (
          <ActivityIndicator color="#38bdf8" />
        ) : domainsQuery.data && domainsQuery.data.length > 0 ? (
          domainsQuery.data.slice(0, 4).map((domain: DomainRecord) => (
            <View key={domain.name || domain.domain} style={styles.domainItem}>
              <Text style={styles.domainName}>{domain.name || domain.domain}</Text>
              <Text style={styles.domainMeta}>
                Owner {truncateAddress(domain.owner || walletAddress || '—')} · Expires{' '}
                {domain.expires_at ? new Date(domain.expires_at).toLocaleDateString() : 'unknown'}
              </Text>
            </View>
          ))
        ) : (
          <Text style={styles.placeholder}>No domains found for this wallet.</Text>
        )}

        <Text style={styles.sectionHeading}>Registry updates</Text>
        {mockDomainUpdates.map((item) => (
          <View key={item.id} style={styles.updateItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.updateTitle}>{item.domain}</Text>
              <Text style={styles.updateDescription}>{item.description}</Text>
              <Text style={styles.updateMeta}>{formatDateTime(item.timestamp)}</Text>
            </View>
            <Tag
              label={item.status === 'completed' ? 'Completed' : item.status === 'pending' ? 'Pending' : 'Failed'}
              tone={item.status === 'completed' ? 'success' : item.status === 'pending' ? 'warning' : 'danger'}
            />
          </View>
        ))}
      </Card>

      <Card title="DNS updates" subtitle="Recent resolver adjustments propagated to IPPAN edge nodes.">
        {mockDnsUpdates.map((item) => (
          <View key={item.id} style={styles.updateItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.updateTitle}>{item.domain}</Text>
              <Text style={styles.updateDescription}>
                {item.recordType} · {item.recordName} → {item.newValue}
              </Text>
              <Text style={styles.updateMeta}>{formatDateTime(item.timestamp)}</Text>
            </View>
            <Tag
              label={item.status === 'completed' ? 'Applied' : item.status === 'pending' ? 'Pending' : 'Failed'}
              tone={item.status === 'completed' ? 'success' : item.status === 'pending' ? 'warning' : 'danger'}
            />
          </View>
        ))}
      </Card>

      <Card title="Storage & availability" subtitle="Plan limits and decentralised storage footprint.">
        <InfoRow label="Reserved" value={`${STORAGE_SAMPLE.reservedGb} GB`} />
        <InfoRow label="Used" value={`${STORAGE_SAMPLE.usedGb} GB`} />
        <InfoRow label="Replication" value={`${STORAGE_SAMPLE.replication}x`} />
        <InfoRow label="Pinned files" value={STORAGE_SAMPLE.pinnedCount} />
        <InfoRow label="Bandwidth used" value={`${STORAGE_SAMPLE.bandwidthUsedGb} GB`} />
        <InfoRow label="Uptime" value={formatPercent(STORAGE_SAMPLE.uptimePct)} />

        <Text style={styles.sectionHeading}>Node availability</Text>
        {AVAILABILITY_SAMPLE.map((item) => (
          <View key={item.node} style={styles.availabilityItem}>
            <View>
              <Text style={styles.updateTitle}>{item.node}</Text>
              <Text style={styles.updateMeta}>{item.region}</Text>
            </View>
            <Text style={styles.transactionAmount}>{formatPercent(item.availability)}</Text>
          </View>
        ))}
      </Card>

      <Card title="TLD insights" subtitle="Changes to premium namespaces and governance notices.">
        {mockTldUpdates.map((item) => (
          <View key={item.id} style={styles.updateItem}>
            <View style={{ flex: 1 }}>
              <Text style={styles.updateTitle}>{item.tld}</Text>
              <Text style={styles.updateDescription}>{item.description}</Text>
              <Text style={styles.updateMeta}>{formatDateTime(item.timestamp)}</Text>
            </View>
            <Tag label={item.status === 'active' ? 'Active' : 'Inactive'} tone={item.status === 'active' ? 'info' : 'neutral'} />
          </View>
        ))}
      </Card>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#0f172a'
  },
  content: {
    padding: 20,
    paddingBottom: 120
  },
  heading: {
    fontSize: 28,
    fontWeight: '700',
    color: '#f8fafc',
    marginBottom: 4
  },
  subheading: {
    fontSize: 15,
    color: '#94a3b8',
    marginBottom: 20
  },
  addressText: {
    fontSize: 13,
    color: '#e2e8f0',
    fontFamily: 'Courier'
  },
  inputRow: {
    marginTop: 6,
    marginBottom: 12
  },
  input: {
    backgroundColor: '#0b1220',
    borderRadius: 12,
    borderWidth: 1,
    borderColor: '#1f2a44',
    paddingHorizontal: 14,
    paddingVertical: 12,
    color: '#f8fafc',
    fontSize: 15
  },
  textArea: {
    minHeight: 96,
    textAlignVertical: 'top'
  },
  buttonRow: {
    flexDirection: 'row',
    gap: 10,
    flexWrap: 'wrap'
  },
  assetList: {
    gap: 12
  },
  assetItem: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12
  },
  assetSymbol: {
    color: '#f8fafc',
    fontSize: 16,
    fontWeight: '700'
  },
  assetName: {
    color: '#94a3b8',
    fontSize: 13
  },
  assetBalance: {
    color: '#38bdf8',
    fontSize: 17,
    fontWeight: '700'
  },
  placeholder: {
    color: '#64748b',
    fontSize: 14
  },
  label: {
    color: '#cbd5f5',
    fontSize: 13,
    marginBottom: 6,
    marginTop: 6
  },
  transactionItem: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 14,
    paddingVertical: 12,
    marginBottom: 12
  },
  transactionTitle: {
    color: '#f8fafc',
    fontWeight: '600',
    fontSize: 15
  },
  transactionSubtitle: {
    color: '#94a3b8',
    fontSize: 13
  },
  transactionMeta: {
    color: '#64748b',
    fontSize: 12,
    marginTop: 4
  },
  transactionAmount: {
    color: '#38bdf8',
    fontWeight: '700',
    fontSize: 16
  },
  transactionFee: {
    color: '#94a3b8',
    fontSize: 12
  },
  domainItem: {
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 14,
    paddingVertical: 12,
    marginBottom: 12
  },
  domainName: {
    color: '#f8fafc',
    fontSize: 15,
    fontWeight: '600'
  },
  domainMeta: {
    color: '#64748b',
    fontSize: 12,
    marginTop: 4
  },
  sectionHeading: {
    color: '#cbd5f5',
    fontSize: 14,
    fontWeight: '600',
    marginTop: 12,
    marginBottom: 8
  },
  updateItem: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 12,
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 14,
    paddingVertical: 12,
    marginBottom: 10
  },
  updateTitle: {
    color: '#f8fafc',
    fontWeight: '600',
    fontSize: 15
  },
  updateDescription: {
    color: '#94a3b8',
    fontSize: 13,
    marginTop: 2
  },
  updateMeta: {
    color: '#64748b',
    fontSize: 12,
    marginTop: 4
  },
  availabilityItem: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#0b1220',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12,
    marginBottom: 10
  }
});
