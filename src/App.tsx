import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import AppShell from '@/components/layout/AppShell'
import LibraryPage from '@/pages/LibraryPage'
import ActorsPage from '@/pages/ActorsPage'
import SeriesPage from '@/pages/SeriesPage'
import TagsPage from '@/pages/TagsPage'
import MakersPage from '@/pages/MakersPage'
import SettingsPage from '@/pages/SettingsPage'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<AppShell />}>
          <Route index element={<Navigate to="/library" replace />} />
          <Route path="library" element={<LibraryPage />} />
          <Route path="library/:id" element={<LibraryPage />} />
          <Route path="actors" element={<ActorsPage />} />
          <Route path="series" element={<SeriesPage />} />
          <Route path="tags" element={<TagsPage />} />
          <Route path="makers" element={<MakersPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}
