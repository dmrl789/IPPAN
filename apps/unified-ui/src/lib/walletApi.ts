import axios from 'axios';

// Get the current API base URL
function getApiBaseUrl(): string {
  return (window as any).API_BASE_URL || 'http://localhost:8080';
}

// Wallet API functions
export async function getWalletBalance(address: string): Promise<{
  account: string;
  balance: number;
  staked: number;
  nonce: number;
}> {
  try {
    const response = await axios.get(`${getApiBaseUrl()}/api/v1/balance`, {
      params: { address }
    });
    return response.data;
  } catch (error) {
    console.error('Error fetching wallet balance:', error);
    return {
      account: address,
      balance: 0,
      staked: 0,
      nonce: 0
    };
  }
}

export async function getWalletTransactions(address: string): Promise<any[]> {
  try {
    const response = await axios.get(`${getApiBaseUrl()}/api/v1/transactions`, {
      params: { address }
    });
    return response.data.transactions || [];
  } catch (error) {
    console.error('Error fetching wallet transactions:', error);
    return [];
  }
}

export async function submitTransaction(transaction: {
  from: string;
  to: string;
  amount: number;
  fee: number;
  nonce: number;
  signature: string;
}): Promise<{ success: boolean; tx_id?: string; message?: string }> {
  try {
    const response = await axios.post(`${getApiBaseUrl()}/api/v1/transaction`, transaction);
    const body = response.data;

    if (body?.success) {
      return {
        success: true,
        tx_id: body?.data?.tx_hash,
      };
    }

    return {
      success: false,
      message: body?.error || 'Failed to submit transaction',
    };
  } catch (error) {
    console.error('Error submitting transaction:', error);
    return {
      success: false,
      message: 'Failed to submit transaction'
    };
  }
}

export async function getNetworkStatus(): Promise<{
  node: {
    is_running: boolean;
    uptime_seconds: number;
    version: string;
    node_id: string;
  };
  network: {
    connected_peers: number;
    known_peers: number;
    total_peers: number;
  };
  mempool: {
    total_transactions: number;
    pending_transactions: number;
  };
  blockchain: {
    current_height: number;
    total_blocks: number;
    total_transactions: number;
  };
}> {
  try {
    const response = await axios.get(`${getApiBaseUrl()}/api/v1/status`);
    return response.data;
  } catch (error) {
    console.error('Error fetching network status:', error);
    return {
      node: {
        is_running: false,
        uptime_seconds: 0,
        version: 'unknown',
        node_id: 'unknown'
      },
      network: {
        connected_peers: 0,
        known_peers: 0,
        total_peers: 0
      },
      mempool: {
        total_transactions: 0,
        pending_transactions: 0
      },
      blockchain: {
        current_height: 0,
        total_blocks: 0,
        total_transactions: 0
      }
    };
  }
}

export async function validateAddress(address: string): Promise<boolean> {
  try {
    const response = await axios.get(`${getApiBaseUrl()}/api/v1/address/validate`, {
      params: { address }
    });
    return response.data.valid || false;
  } catch (error) {
    console.error('Error validating address:', error);
    return false;
  }
}
