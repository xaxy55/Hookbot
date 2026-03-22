import { useState, useEffect, type ReactNode } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { HotkeysProvider } from '@tanstack/react-hotkeys';
import { ToastProvider } from './components/Toast';
import { getAuthStatus } from './api/client';
import { HotkeyContext, useHotkeySetup, useNavigationHotkeys } from './hooks/useHotkeys';
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
import PetCarePage from './pages/PetCarePage';
import MoodJournalPage from './pages/MoodJournalPage';
import PomodoroPage from './pages/PomodoroPage';
import SoundBoardPage from './pages/SoundBoardPage';
import LightShowPage from './pages/LightShowPage';
import LoginPage from './pages/LoginPage';
import DeviceLinksPage from './pages/DeviceLinksPage';
import UsersPage from './pages/UsersPage';
import TunnelsPage from './pages/TunnelsPage';
import MoodLearningPage from './pages/MoodLearningPage';
import VoiceControlPage from './pages/VoiceControlPage';
import SocialPage from './pages/SocialPage';
import TeamDashboardPage from './pages/TeamDashboardPage';
import GlobalWallPage from './pages/GlobalWallPage';
import DeskLightsPage from './pages/DeskLightsPage';
import MusicPage from './pages/MusicPage';
import StandingDeskPage from './pages/StandingDeskPage';
import StreamDeckPage from './pages/StreamDeckPage';
import HomeAssistantPage from './pages/HomeAssistantPage';
import DeskOccupancyPage from './pages/DeskOccupancyPage';
import MonitorsPage from './pages/MonitorsPage';
import DeveloperInsightsPage from './pages/DeveloperInsightsPage';
import AccountPage from './pages/AccountPage';

function AuthGuard({ children }: { children: ReactNode }) {
  const [authed, setAuthed] = useState<boolean | null>(null);

  useEffect(() => {
    getAuthStatus()
      .then(r => setAuthed(r.authenticated))
      .catch(() => setAuthed(false));
  }, []);

  if (authed === null) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-canvas">
        <div className="h-8 w-8 rounded-full border-2 border-brand/30 border-t-brand animate-spin" />
      </div>
    );
  }

  if (!authed) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}

function HotkeyWrapper({ children }: { children: ReactNode }) {
  const ctx = useHotkeySetup();
  useNavigationHotkeys();
  return (
    <HotkeyContext.Provider value={ctx}>
      {children}
    </HotkeyContext.Provider>
  );
}

export default function App() {
  return (
    <HotkeysProvider>
    <ToastProvider>
    <BrowserRouter>
      <HotkeyWrapper>
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
          <Route path="/pet" element={<PetCarePage />} />
          <Route path="/mood" element={<MoodJournalPage />} />
          <Route path="/pomodoro" element={<PomodoroPage />} />
          <Route path="/sounds" element={<SoundBoardPage />} />
          <Route path="/lights" element={<LightShowPage />} />
          <Route path="/store" element={<StorePage />} />
          <Route path="/community" element={<CommunityStorePage />} />
          <Route path="/shared-assets" element={<AssetSharingPage />} />
          <Route path="/device-links" element={<DeviceLinksPage />} />
          <Route path="/users" element={<UsersPage />} />
          <Route path="/tunnels" element={<TunnelsPage />} />
          <Route path="/mood-learning" element={<MoodLearningPage />} />
          <Route path="/voice" element={<VoiceControlPage />} />
          <Route path="/social" element={<SocialPage />} />
          <Route path="/team" element={<TeamDashboardPage />} />
          <Route path="/global-wall" element={<GlobalWallPage />} />
          <Route path="/desk-lights" element={<DeskLightsPage />} />
          <Route path="/music" element={<MusicPage />} />
          <Route path="/standing-desk" element={<StandingDeskPage />} />
          <Route path="/streamdeck" element={<StreamDeckPage />} />
          <Route path="/homeassistant" element={<HomeAssistantPage />} />
          <Route path="/desk-occupancy" element={<DeskOccupancyPage />} />
          <Route path="/monitors" element={<MonitorsPage />} />
          <Route path="/insights" element={<DeveloperInsightsPage />} />
          <Route path="/integrations" element={<IntegrationsPage />} />
          <Route path="/account" element={<AccountPage />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route path="/diagnostics" element={<DiagnosticsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
      </HotkeyWrapper>
    </BrowserRouter>
    </ToastProvider>
    </HotkeysProvider>
  );
}
