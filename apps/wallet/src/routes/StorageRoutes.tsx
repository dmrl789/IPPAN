import { Route, Routes, Navigate } from 'react-router-dom'
import UploadPage from '../pages/storage/UploadPage'

export default function StorageRoutes() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/storage/files" replace />} />
      <Route path="files" element={<div style={{padding:16}}>Files (encrypt → shard → upload)</div>} />
      <Route path="upload" element={<UploadPage />} />
      <Route path="txt" element={<div style={{padding:16}}>TXT / Server Info</div>} />
      <Route path="dht" element={<div style={{padding:16}}>DHT Explorer</div>} />
    </Routes>
  )
}
