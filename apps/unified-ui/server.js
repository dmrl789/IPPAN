import express from 'express';
import cors from 'cors';
import multer from 'multer';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const app = express();
const PORT = 3001; // Run on different port than frontend

// Middleware
app.use(cors());
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Multer configuration for file uploads
const storage = multer.diskStorage({
  destination: (req, file, cb) => {
    const uploadDir = 'uploads/';
    if (!fs.existsSync(uploadDir)) {
      fs.mkdirSync(uploadDir, { recursive: true });
    }
    cb(null, uploadDir);
  },
  filename: (req, file, cb) => {
    const uniqueSuffix = Date.now() + '-' + Math.round(Math.random() * 1E9);
    cb(null, file.fieldname + '-' + uniqueSuffix + path.extname(file.originalname));
  }
});

const upload = multer({ storage: storage });

// Mock data storage
let files = [];
let transactions = [];
let validators = [
  {
    address: "i1validator1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    stake: 1000000,
    status: "active"
  },
  {
    address: "i1validator2345678901bcdef1234567890abcdef1234567890abcdef1234567890",
    stake: 850000,
    status: "active"
  }
];

// Health and Status Endpoints
app.get('/health', (req, res) => {
  res.json({
    status: "healthy",
    timestamp: new Date().toISOString(),
    version: "1.0.0",
    uptime: process.uptime(),
    node_id: "i1node1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  });
});

app.get('/status', (req, res) => {
  res.json({
    node_id: "i1node1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    status: "running",
    current_block: 1234567,
    total_transactions: 9876543,
    network_peers: 42,
    uptime_seconds: Math.floor(process.uptime()),
    version: "1.0.0"
  });
});

// Storage Endpoints
app.post('/storage/files', upload.single('file'), (req, res) => {
  if (!req.file) {
    return res.status(400).json({ error: 'No file uploaded' });
  }

  const fileId = `file_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  const fileInfo = {
    file_id: fileId,
    name: req.file.originalname,
    size: req.file.size,
    hash: `sha256_${Math.random().toString(36).substr(2, 16)}`,
    uploaded_at: new Date().toISOString()
  };

  files.push(fileInfo);
  
  res.status(200).json({
    file_id: fileId,
    hash: fileInfo.hash,
    size: req.file.size
  });
});

app.get('/storage/files', (req, res) => {
  res.json(files);
});

app.get('/storage/files/:fileId', (req, res) => {
  const fileId = req.params.fileId;
  const file = files.find(f => f.file_id === fileId);
  
  if (!file) {
    return res.status(404).json({ error: 'File not found' });
  }

  // In a real implementation, you would stream the actual file
  res.json({
    file_id: file.file_id,
    name: file.name,
    size: file.size,
    hash: file.hash,
    content: `Mock content for file ${file.name}`
  });
});

app.delete('/storage/files/:fileId', (req, res) => {
  const fileId = req.params.fileId;
  const fileIndex = files.findIndex(f => f.file_id === fileId);
  
  if (fileIndex === -1) {
    return res.status(404).json({ error: 'File not found' });
  }

  files.splice(fileIndex, 1);
  res.status(200).json({ message: 'File deleted successfully' });
});

// Wallet Endpoints
app.get('/wallet/balance', (req, res) => {
  res.json({
    address: "i1wallet1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    balance: "1000.0",
    staked_amount: "100.0",
    rewards: "5.25",
    pending_transactions: []
  });
});

app.post('/wallet/send', (req, res) => {
  const { to, amount, fee } = req.body;
  
  if (!to || !amount) {
    return res.status(400).json({ error: 'Missing required fields: to, amount' });
  }

  const txHash = `tx_${Date.now()}_${Math.random().toString(36).substr(2, 16)}`;
  const transaction = {
    hash: txHash,
    to: to,
    amount: amount,
    fee: fee || "0.01",
    status: "pending",
    timestamp: new Date().toISOString()
  };

  transactions.push(transaction);
  
  res.status(200).json({
    hash: txHash,
    status: "pending"
  });
});

// Consensus Endpoints
app.get('/consensus/round', (req, res) => {
  res.json({
    round_number: 1234567,
    validators: validators.map(v => v.address),
    status: "active",
    timestamp: new Date().toISOString()
  });
});

app.get('/consensus/validators', (req, res) => {
  res.json(validators);
});

// Error handling middleware
app.use((err, req, res, next) => {
  console.error(err.stack);
  res.status(500).json({ error: 'Something went wrong!' });
});

// 404 handler
app.use((req, res) => {
  res.status(404).json({ error: 'Endpoint not found' });
});

// Start server
app.listen(PORT, () => {
  console.log(`🚀 IPPAN API Server running on http://localhost:${PORT}`);
  console.log(`📋 Available endpoints:`);
  console.log(`   GET  /health`);
  console.log(`   GET  /status`);
  console.log(`   POST /storage/files`);
  console.log(`   GET  /storage/files`);
  console.log(`   GET  /storage/files/:fileId`);
  console.log(`   DELETE /storage/files/:fileId`);
  console.log(`   GET  /wallet/balance`);
  console.log(`   POST /wallet/send`);
  console.log(`   GET  /consensus/round`);
  console.log(`   GET  /consensus/validators`);
});

export default app;
