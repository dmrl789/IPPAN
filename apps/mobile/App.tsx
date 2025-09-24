import 'react-native-gesture-handler';
import React from 'react';
import { NavigationContainer, DefaultTheme, Theme } from '@react-navigation/native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { StatusBar } from 'expo-status-bar';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Ionicons } from '@expo/vector-icons';

import WalletOverviewScreen from './src/screens/wallet/WalletOverviewScreen';
import ExplorerScreen from './src/screens/explorer/ExplorerScreen';
import NeuralScreen from './src/screens/neural/NeuralScreen';
import NodeManagementScreen from './src/screens/nodes/NodeManagementScreen';
import { ApiProvider } from './src/providers/ApiProvider';
import { WalletProvider } from './src/providers/WalletProvider';

const Tab = createBottomTabNavigator();

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60 * 1000,
      refetchOnWindowFocus: false
    }
  }
});

const navigationTheme: Theme = {
  ...DefaultTheme,
  colors: {
    ...DefaultTheme.colors,
    background: '#0f172a',
    card: '#111c33',
    text: '#f8fafc',
    border: '#1f2a44',
    notification: '#38bdf8',
    primary: '#38bdf8'
  }
};

export default function App() {
  return (
    <SafeAreaProvider>
      <ApiProvider>
        <WalletProvider>
          <QueryClientProvider client={queryClient}>
            <NavigationContainer theme={navigationTheme}>
              <StatusBar style="light" />
              <Tab.Navigator
                screenOptions={({ route }) => ({
                  headerShown: false,
                  tabBarStyle: {
                    backgroundColor: '#111c33',
                    borderTopColor: '#1f2a44',
                    paddingBottom: 6,
                    paddingTop: 6,
                    height: 64
                  },
                  tabBarActiveTintColor: '#38bdf8',
                  tabBarInactiveTintColor: '#94a3b8',
                  tabBarIcon: ({ color, size }) => {
                    let iconName: keyof typeof Ionicons.glyphMap = 'apps';

                    if (route.name === 'Wallet') {
                      iconName = 'wallet-outline';
                    } else if (route.name === 'Explorer') {
                      iconName = 'planet-outline';
                    } else if (route.name === 'Neural') {
                      iconName = 'hardware-chip-outline';
                    } else if (route.name === 'Nodes') {
                      iconName = 'settings-outline';
                    }

                    return <Ionicons name={iconName} size={size} color={color} />;
                  }
                })}
              >
                <Tab.Screen name="Wallet" component={WalletOverviewScreen} />
                <Tab.Screen name="Explorer" component={ExplorerScreen} />
                <Tab.Screen name="Neural" component={NeuralScreen} />
                <Tab.Screen name="Nodes" component={NodeManagementScreen} />
              </Tab.Navigator>
            </NavigationContainer>
          </QueryClientProvider>
        </WalletProvider>
      </ApiProvider>
    </SafeAreaProvider>
  );
}
