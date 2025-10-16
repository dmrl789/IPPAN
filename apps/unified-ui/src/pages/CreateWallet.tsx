import { useState } from 'react'
import { Card, Button, Field, Input, Badge } from '../components/UI'
import { generateWallet, validateAddress, deriveAddressFromSeed } from '../lib/crypto'

interface WalletCreationResult {
  privateKey: string
  publicKey: string
  address: string
  seedPhrase: string
}

export default function CreateWallet() {
  const [wallet, setWallet] = useState<WalletCreationResult | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [showPrivateKey, setShowPrivateKey] = useState(false)
  const [showSeedPhrase, setShowSeedPhrase] = useState(false)
  const [importMode, setImportMode] = useState(false)
  const [importSeedPhrase, setImportSeedPhrase] = useState('')

  const handleCreateWallet = async () => {
    setLoading(true)
    setError(null)
    
    try {
      // Simulate some processing time
      await new Promise(resolve => setTimeout(resolve, 1000))
      
      const newWallet = await generateWallet()
      setWallet(newWallet)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create wallet')
    } finally {
      setLoading(false)
    }
  }

  const handleImportWallet = async () => {
    setLoading(true)
    setError(null)
    
    try {
      if (!importSeedPhrase.trim()) {
        throw new Error('Seed phrase is required')
      }

      const words = importSeedPhrase.trim().split(/\s+/)
      if (words.length !== 12) {
        throw new Error('Seed phrase must contain exactly 12 words')
      }

      // Simulate some processing time
      await new Promise(resolve => setTimeout(resolve, 1000))
      
      const address = await deriveAddressFromSeed(importSeedPhrase.trim())
      const publicKey = address // Simplified for demo
      const privateKey = 'imported_' + Math.random().toString(36).substr(2, 9)
      
      setWallet({
        privateKey,
        publicKey,
        address,
        seedPhrase: importSeedPhrase.trim()
      })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to import wallet')
    } finally {
      setLoading(false)
    }
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
  }

  const downloadWallet = () => {
    if (!wallet) return
    
    const walletData = {
      address: wallet.address,
      publicKey: wallet.publicKey,
      privateKey: wallet.privateKey,
      seedPhrase: wallet.seedPhrase,
      created: new Date().toISOString()
    }
    
    const blob = new Blob([JSON.stringify(walletData, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `ippan-wallet-${wallet.address.slice(-8)}.json`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Create Wallet</h1>
        <div className="flex space-x-2">
          <Button 
            variant={!importMode ? "default" : "outline"}
            onClick={() => setImportMode(false)}
          >
            Create New
          </Button>
          <Button 
            variant={importMode ? "default" : "outline"}
            onClick={() => setImportMode(true)}
          >
            Import Existing
          </Button>
        </div>
      </div>

      {!wallet ? (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Create New Wallet */}
          {!importMode && (
            <Card>
              <h3 className="text-lg font-semibold mb-4">Create New Wallet</h3>
              <div className="space-y-4">
                <div className="p-4 bg-blue-50 border border-blue-200 rounded-lg">
                  <h4 className="font-semibold text-blue-900 mb-2">Security Notice</h4>
                  <ul className="text-sm text-blue-800 space-y-1">
                    <li>• Your private key and seed phrase will be generated</li>
                    <li>• Save them securely - they cannot be recovered</li>
                    <li>• Never share your private key or seed phrase</li>
                    <li>• This wallet is generated locally in your browser</li>
                  </ul>
                </div>
                
                <Button 
                  onClick={handleCreateWallet}
                  disabled={loading}
                  className="w-full"
                >
                  {loading ? 'Generating...' : 'Generate New Wallet'}
                </Button>
              </div>
            </Card>
          )}

          {/* Import Existing Wallet */}
          {importMode && (
            <Card>
              <h3 className="text-lg font-semibold mb-4">Import Existing Wallet</h3>
              <form onSubmit={(e) => { e.preventDefault(); handleImportWallet(); }} className="space-y-4">
                <Field label="Seed Phrase (12 words)" required>
                  <textarea
                    className="w-full p-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    rows={3}
                    placeholder="Enter your 12-word seed phrase..."
                    value={importSeedPhrase}
                    onChange={(e) => setImportSeedPhrase(e.target.value)}
                  />
                </Field>
                
                <Button 
                  type="submit"
                  disabled={loading || !importSeedPhrase.trim()}
                  className="w-full"
                >
                  {loading ? 'Importing...' : 'Import Wallet'}
                </Button>
              </form>
            </Card>
          )}

          {/* Security Tips */}
          <Card>
            <h3 className="text-lg font-semibold mb-4">Security Tips</h3>
            <div className="space-y-3 text-sm">
              <div className="flex items-start space-x-2">
                <span className="text-red-500">⚠️</span>
                <span>Never share your private key or seed phrase with anyone</span>
              </div>
              <div className="flex items-start space-x-2">
                <span className="text-yellow-500">🔒</span>
                <span>Store your seed phrase in a secure, offline location</span>
              </div>
              <div className="flex items-start space-x-2">
                <span className="text-green-500">✅</span>
                <span>Use hardware wallets for large amounts</span>
              </div>
              <div className="flex items-start space-x-2">
                <span className="text-blue-500">💡</span>
                <span>Test with small amounts first</span>
              </div>
            </div>
          </Card>
        </div>
      ) : (
        /* Wallet Created Successfully */
        <div className="space-y-6">
          <Card className="border-green-200 bg-green-50">
            <div className="flex items-center space-x-2 mb-4">
              <span className="text-green-600 text-2xl">✅</span>
              <h3 className="text-lg font-semibold text-green-900">Wallet Created Successfully!</h3>
            </div>
            <p className="text-green-800">
              Your new IPPAN wallet has been generated. Please save your private key and seed phrase securely.
            </p>
          </Card>

          {/* Wallet Address */}
          <Card>
            <h3 className="text-lg font-semibold mb-4">Wallet Address</h3>
            <div className="space-y-3">
              <div className="p-3 bg-gray-100 rounded-lg">
                <div className="font-mono text-sm break-all">{wallet.address}</div>
              </div>
              <div className="flex space-x-2">
                <Button 
                  size="sm" 
                  variant="outline"
                  onClick={() => copyToClipboard(wallet.address)}
                >
                  Copy Address
                </Button>
                <Button 
                  size="sm" 
                  variant="outline"
                  onClick={() => setWallet(null)}
                >
                  Create Another
                </Button>
              </div>
            </div>
          </Card>

          {/* Seed Phrase */}
          <Card>
            <h3 className="text-lg font-semibold mb-4">Seed Phrase (12 words)</h3>
            <div className="space-y-3">
              <div className="p-3 bg-yellow-50 border border-yellow-200 rounded-lg">
                <div className="text-yellow-800 text-sm font-semibold mb-2">
                  ⚠️ CRITICAL: Save this seed phrase securely!
                </div>
                <div className="text-yellow-700 text-sm">
                  Anyone with this seed phrase can access your wallet. Write it down and store it safely.
                </div>
              </div>
              
              <div className="p-3 bg-gray-100 rounded-lg">
                <div className="font-mono text-sm">
                  {showSeedPhrase ? wallet.seedPhrase : '••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• •••'}
                </div>
              </div>
              
              <div className="flex space-x-2">
                <Button 
                  size="sm" 
                  variant="outline"
                  onClick={() => setShowSeedPhrase(!showSeedPhrase)}
                >
                  {showSeedPhrase ? 'Hide' : 'Show'} Seed Phrase
                </Button>
                <Button 
                  size="sm" 
                  variant="outline"
                  onClick={() => copyToClipboard(wallet.seedPhrase)}
                >
                  Copy Seed Phrase
                </Button>
              </div>
            </div>
          </Card>

          {/* Private Key */}
          <Card>
            <h3 className="text-lg font-semibold mb-4">Private Key</h3>
            <div className="space-y-3">
              <div className="p-3 bg-red-50 border border-red-200 rounded-lg">
                <div className="text-red-800 text-sm font-semibold mb-2">
                  🔐 PRIVATE: Never share this key!
                </div>
                <div className="text-red-700 text-sm">
                  This private key gives full access to your wallet. Keep it secret.
                </div>
              </div>
              
              <div className="p-3 bg-gray-100 rounded-lg">
                <div className="font-mono text-sm break-all">
                  {showPrivateKey ? wallet.privateKey : '••••••••••••••••••••••••••••••••••••••••••••••••••••••••••••••••'}
                </div>
              </div>
              
              <div className="flex space-x-2">
                <Button 
                  size="sm" 
                  variant="outline"
                  onClick={() => setShowPrivateKey(!showPrivateKey)}
                >
                  {showPrivateKey ? 'Hide' : 'Show'} Private Key
                </Button>
                <Button 
                  size="sm" 
                  variant="outline"
                  onClick={() => copyToClipboard(wallet.privateKey)}
                >
                  Copy Private Key
                </Button>
              </div>
            </div>
          </Card>

          {/* Actions */}
          <Card>
            <h3 className="text-lg font-semibold mb-4">Wallet Actions</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <Button 
                onClick={downloadWallet}
                className="w-full"
              >
                📥 Download Wallet File
              </Button>
              <Button 
                variant="outline"
                onClick={() => {
                  setWallet(null)
                  setImportMode(false)
                  setImportSeedPhrase('')
                }}
                className="w-full"
              >
                🔄 Create New Wallet
              </Button>
            </div>
          </Card>
        </div>
      )}

      {/* Error Display */}
      {error && (
        <Card className="border-red-200 bg-red-50">
          <div className="text-red-800">
            <strong>Error:</strong> {error}
          </div>
        </Card>
      )}
    </div>
  )
}
