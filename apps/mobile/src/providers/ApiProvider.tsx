import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { View, ActivityIndicator, StyleSheet } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import axios, { AxiosInstance } from 'axios';
import Constants from 'expo-constants';

const STORAGE_KEY = 'ippan.mobile.api.baseUrl';

type ConstantsWithManifest = typeof Constants & {
  manifest?: {
    extra?: Record<string, unknown>;
  };
};

const expoConfigExtra = (Constants?.expoConfig?.extra ?? null) as Record<string, unknown> | null;
const legacyExtra = (Constants as ConstantsWithManifest).manifest?.extra ?? null;
const extra = expoConfigExtra ?? legacyExtra ?? {};
const DEFAULT_BASE_URL = typeof extra.defaultApiBaseUrl === 'string' ? extra.defaultApiBaseUrl : 'http://localhost:8080';

export interface ApiContextValue {
  baseUrl: string;
  client: AxiosInstance;
  setBaseUrl: (url: string) => void;
  isReady: boolean;
}

const ApiContext = createContext<ApiContextValue | undefined>(undefined);

function normalizeUrl(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) {
    return DEFAULT_BASE_URL;
  }
  if (/^https?:\/\//i.test(trimmed)) {
    return trimmed.replace(/\/$/, '');
  }
  return `http://${trimmed.replace(/\/$/, '')}`;
}

export const ApiProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [baseUrl, setBaseUrlState] = useState<string>(DEFAULT_BASE_URL);
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    let mounted = true;

    AsyncStorage.getItem(STORAGE_KEY)
      .then((stored) => {
        if (stored && mounted) {
          setBaseUrlState(stored);
        }
      })
      .catch((error) => {
        console.warn('Failed to load API base URL from storage', error);
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

  const setBaseUrl = useCallback((url: string) => {
    const normalized = normalizeUrl(url);
    setBaseUrlState(normalized);
    AsyncStorage.setItem(STORAGE_KEY, normalized).catch((error) => {
      console.warn('Failed to persist API base URL', error);
    });
  }, []);

  const client = useMemo(() => {
    const instance = axios.create({
      baseURL: baseUrl,
      headers: {
        'Content-Type': 'application/json'
      },
      timeout: 12_000
    });

    return instance;
  }, [baseUrl]);

  const value = useMemo<ApiContextValue>(
    () => ({
      baseUrl,
      client,
      setBaseUrl,
      isReady
    }),
    [baseUrl, client, setBaseUrl, isReady]
  );

  if (!isReady) {
    return (
      <View style={styles.loadingContainer}>
        <ActivityIndicator color="#38bdf8" size="large" />
      </View>
    );
  }

  return <ApiContext.Provider value={value}>{children}</ApiContext.Provider>;
};

export function useApi(): ApiContextValue {
  const context = useContext(ApiContext);
  if (!context) {
    throw new Error('useApi must be used within an ApiProvider');
  }
  return context;
}

const styles = StyleSheet.create({
  loadingContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: '#0f172a'
  }
});
