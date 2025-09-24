import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as Crypto from 'expo-crypto';

export type WalletType = 'watch-only' | 'local';

interface WalletContextValue {
  walletAddress: string | null;
  walletType: WalletType;
  isReady: boolean;
  setWatchAddress: (address: string) => Promise<void>;
  connectLocalWallet: () => Promise<string>;
  disconnectWallet: () => Promise<void>;
  signMessage: (message: string) => Promise<string>;
}

const WalletContext = createContext<WalletContextValue | undefined>(undefined);

const ADDRESS_KEY = 'ippan.wallet.address';
const TYPE_KEY = 'ippan.wallet.type';

const ADDRESS_REGEX = /^i[0-9a-fA-F]{64}$/;

function randomHex(length: number): string {
  let output = '';
  while (output.length < length) {
    output += Math.floor(Math.random() * 16).toString(16);
  }
  return output.slice(0, length);
}

export const WalletProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [walletAddress, setWalletAddress] = useState<string | null>(null);
  const [walletType, setWalletType] = useState<WalletType>('watch-only');
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    let mounted = true;

    AsyncStorage.multiGet([ADDRESS_KEY, TYPE_KEY])
      .then((entries) => {
        if (!mounted) return;
        const storedAddress = entries[0]?.[1];
        const storedType = (entries[1]?.[1] as WalletType | null) || 'watch-only';
        if (storedAddress) {
          setWalletAddress(storedAddress);
        }
        setWalletType(storedType);
      })
      .catch((error) => {
        console.warn('Failed to load wallet state', error);
      })
      .finally(() => {
        if (mounted) {
          setIsReady(true);
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  const persistState = useCallback(async (address: string | null, type: WalletType) => {
    setWalletAddress(address);
    setWalletType(type);
    if (address) {
      await AsyncStorage.multiSet([
        [ADDRESS_KEY, address],
        [TYPE_KEY, type]
      ]);
    } else {
      await AsyncStorage.removeItem(ADDRESS_KEY);
      await AsyncStorage.setItem(TYPE_KEY, type);
    }
  }, []);

  const setWatchAddress = useCallback(
    async (address: string) => {
      const normalized = address.trim();
      if (!ADDRESS_REGEX.test(normalized)) {
        throw new Error('Addresses must start with i and contain 64 hexadecimal characters.');
      }
      await persistState(normalized, 'watch-only');
    },
    [persistState]
  );

  const connectLocalWallet = useCallback(async () => {
    const generatedAddress = `i${randomHex(64)}`;
    await persistState(generatedAddress, 'local');
    return generatedAddress;
  }, [persistState]);

  const disconnectWallet = useCallback(async () => {
    await persistState(null, 'watch-only');
  }, [persistState]);

  const signMessage = useCallback(
    async (message: string) => {
      if (walletType !== 'local' || !walletAddress) {
        throw new Error('Only locally managed wallets can sign messages on mobile.');
      }
      const digest = await Crypto.digestStringAsync(
        Crypto.CryptoDigestAlgorithm.SHA256,
        `${walletAddress}:${message}`
      );
      return `sig_${digest.slice(0, 48)}`;
    },
    [walletAddress, walletType]
  );

  const value = useMemo<WalletContextValue>(
    () => ({
      walletAddress,
      walletType,
      isReady,
      setWatchAddress,
      connectLocalWallet,
      disconnectWallet,
      signMessage
    }),
    [walletAddress, walletType, isReady, setWatchAddress, connectLocalWallet, disconnectWallet, signMessage]
  );

  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
};

export function useWallet(): WalletContextValue {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}
