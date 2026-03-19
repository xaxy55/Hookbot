import { useState, useEffect, type ReactNode } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ToastProvider } from './components/Toast';
import { getAuthStatus } from './api/client';
import Layout from './components/Layout';
import Dashboard from './pages/Dashboard';
import DevicesPage from './pages/DevicesPage';
import DeviceDetail from './pages/DeviceDetail';
import DiscoveryPage from './pages/DiscoveryPage';
import OtaPage from './pages/OtaPage';
import Settings from './pages/Settings';
import LogsPage from './pages/LogsPage';
import DiagnosticsPage from './pages/DiagnosticsPage';
import AvatarEditorPage from './pages/AvatarEditorPage';
import AnimationEditorPage from './pages/AnimationEditorPage';
import IntegrationsPage from './pages/IntegrationsPage';
import ActivityFeedPage from './pages/ActivityFeedPage';
import AnalyticsPage from './pages/AnalyticsPage';
import AchievementsPage from './pages/AchievementsPage';
import BleSetupPage from './pages/BleSetupPage';
import StorePage from './pages/StorePage';
import CommunityStorePage from './pages/CommunityStorePage';
import AssetSharingPage from './pages/AssetSharingPage';
import LoginPage from './pages/LoginPage';

function AuthGuard({ children }: { children: ReactNode }) {
  const [authed, setAuthed] = useState<boolean | null>(null);

  useEffect(() => {
    getAuthStatus()
      .then(r => setAuthed(r.authenticated))
      .catch(() => setAuthed(false));
  }, []);

  if (authed === null) {
    return (
      <div style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#0a0a0f',
        color: '#888',
      }}>
        Loading...
      </div>
    );
  }

  if (!authed) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}

export default function App() {
  return (
    <ToastProvider>
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route element={<AuthGuard><Layout /></AuthGuard>}>
          <Route path="/" element={<Dashboard />} />
          <Route path="/devices" element={<DevicesPage />} />
          <Route path="/devices/:id" element={<DeviceDetail />} />
          <Route path="/discovery" element={<DiscoveryPage />} />
          <Route path="/ble-setup" element={<BleSetupPage />} />
          <Route path="/avatar" element={<AvatarEditorPage />} />
          <Route path="/animations" element={<AnimationEditorPage />} />
          <Route path="/ota" element={<OtaPage />} />
          <Route path="/activity" element={<ActivityFeedPage />} />
          <Route path="/analytics" element={<AnalyticsPage />} />
          <Route path="/achievements" element={<AchievementsPage />} />
          <Route path="/store" element={<StorePage />} />
          <Route path="/community" element={<CommunityStorePage />} />
          <Route path="/shared-assets" element={<AssetSharingPage />} />
          <Route path="/integrations" element={<IntegrationsPage />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route path="/diagnostics" element={<DiagnosticsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
    </ToastProvider>
  );
}
