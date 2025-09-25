import axios from 'axios'
import { getApiBaseUrl } from './api'

export interface WalletBalanceResponse {
  account: string
  address?: string
  balance: number
  staked: number
  staked_amount?: number
  rewards: number
  nonce: number
  pending_transactions: string[]
}

export interface WalletTransaction {
  id: string
  from: string
  to: string
  amount: number
  nonce: number
  timestamp: number
  direction?: string
  hashtimer?: string
}

export interface SubmitTransactionRequest {
  from: string
  to: string
  amount: number
  fee: number
  nonce: number
  signature: string
}

export interface SubmitTransactionResponse {
  success: boolean
  tx_id?: string
  message?: string
}

export async function getWalletBalance(address: string): Promise<WalletBalanceResponse> {
  try {
    const response = await axios.get(`${getApiBaseUrl()}/api/v1/balance`, {
      params: { address },
    })
    const data = response.data as WalletBalanceResponse

    return {
      account: data.account || address,
      address: data.address ?? data.account ?? address,
      balance: data.balance ?? 0,
      staked: data.staked ?? data.staked_amount ?? 0,
      staked_amount: data.staked_amount,
      rewards: data.rewards ?? 0,
      nonce: data.nonce ?? 0,
      pending_transactions: Array.isArray(data.pending_transactions) ? data.pending_transactions : [],
    }
  } catch (error) {
    console.error('Error fetching wallet balance:', error)
    return {
      account: address,
      address,
      balance: 0,
      staked: 0,
      rewards: 0,
      nonce: 0,
      pending_transactions: [],
    }
  }
}

export async function getWalletTransactions(address: string): Promise<WalletTransaction[]> {
  try {
    const response = await axios.get(`${getApiBaseUrl()}/api/v1/transactions`, {
      params: { address },
    })
    const body = response.data
    if (Array.isArray(body?.transactions)) {
      return body.transactions as WalletTransaction[]
    }
    return []
  } catch (error) {
    console.error('Error fetching wallet transactions:', error)
    return []
  }
}

export async function submitTransaction(transaction: SubmitTransactionRequest): Promise<SubmitTransactionResponse> {
  try {
    const response = await axios.post(`${getApiBaseUrl()}/api/v1/transaction`, transaction)
    const body = response.data

    if (body?.success) {
      return {
        success: true,
        tx_id: body?.data?.tx_hash,
      }
    }

    return {
      success: false,
      message: body?.error || 'Failed to submit transaction',
    }
  } catch (error) {
    console.error('Error submitting transaction:', error)
    return {
      success: false,
      message: 'Failed to submit transaction',
    }
  }
}

export async function validateAddress(address: string): Promise<boolean> {
  try {
    const response = await axios.get(`${getApiBaseUrl()}/api/v1/address/validate`, {
      params: { address },
    })
    return Boolean(response.data?.valid)
  } catch (error) {
    console.error('Error validating address:', error)
    throw error
  }
}
