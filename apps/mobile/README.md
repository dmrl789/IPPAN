# IPPAN Unified Mobile

The mobile companion to the [Unified UI](../unified-ui) delivers wallet, explorer, neural-network and node-management tooling on Android and iOS. It is built with Expo + React Native so that a single codebase targets both platforms while sharing the same API contracts as the web dashboard.

## Features

- **Wallet & Finance** – connect a watch-only wallet or generate a local demo wallet, review balances, submit signed transactions, track domains, storage metrics and registry activity.
- **Blockchain Explorer** – monitor node telemetry, peer connectivity, mempool pressure, consensus health and recent block production.
- **Neural Marketplace** – browse IPPAN models and datasets, inspect inference workloads, bids and proof events. Live API responses are used when available with graceful fallbacks for offline demos.
- **Node Management** – switch between validators/gateways, run health checks and pick pre-configured endpoints without leaving the app.

## Getting started

The project lives under `apps/mobile` and uses Expo for development convenience.

```bash
cd apps/mobile
npm install
```

### Development

- **Start the bundler** – `npm start`
- **Run on Android** – `npm run android`
- **Run on iOS** – `npm run ios`

The Expo CLI will guide you through connecting a simulator or physical device. Make sure your machine meets the [Expo tooling prerequisites](https://docs.expo.dev/get-started/installation/).

### Environment configuration

The app reads the active API endpoint from `app.json` (`extra.defaultApiBaseUrl`). You can override it at runtime from the **Nodes** tab:

1. Open the **Nodes** tab.
2. Enter the base URL of the validator or gateway (for example `http://localhost:8080`).
3. Press **Apply** or choose one of the curated endpoints.

The selection persists in `AsyncStorage`, mirroring the behaviour of the web UI.

### Shared behaviour with the web client

- Wallet and node state are persisted across restarts just like the `localStorage` strategy in `apps/unified-ui`.
- API helpers (`fetchWalletBalance`, `fetchNodeStatus`, …) mimic the TypeScript contracts used on the web and gracefully down-level to empty data when a node does not expose a specific endpoint.
- Mock/fallback data mirrors the content shown in the desktop experience when real endpoints are unavailable.

### Linting & type checking

```bash
npm run lint
npm run typecheck
```

Both commands are recommended before committing changes.

## Project structure

```
apps/mobile
├── App.tsx                # Entry point with navigation + providers
├── app.json               # Expo configuration (name, slug, default API URL)
├── src/
│   ├── api/               # Axios-powered API clients shared across screens
│   ├── components/        # Reusable UI primitives (Card, Button, etc.)
│   ├── data/              # Mock data used when endpoints are offline
│   ├── providers/         # API + Wallet context providers (AsyncStorage backed)
│   ├── screens/           # Wallet, Explorer, Neural and Nodes tabs
│   └── utils/             # Formatting helpers
└── README.md
```

## Production builds

Once the Expo project is configured with an EAS account you can produce native binaries with:

```bash
npx expo run:android --variant release
npx expo run:ios --configuration Release
```

Refer to the [Expo build documentation](https://docs.expo.dev/build/introduction/) for signing and store submission guidance.
