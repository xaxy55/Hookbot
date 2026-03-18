import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
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

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
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
          <Route path="/integrations" element={<IntegrationsPage />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route path="/diagnostics" element={<DiagnosticsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
