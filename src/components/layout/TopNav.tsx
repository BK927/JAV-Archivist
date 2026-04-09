import { NavLink, useNavigate } from 'react-router-dom'
import { Input } from '@/components/ui/input'
import { useLibraryStore } from '@/stores/libraryStore'
import { Search } from 'lucide-react'

const TABS = [
  { path: '/library', label: '라이브러리' },
  { path: '/actors', label: '배우' },
  { path: '/series', label: '시리즈' },
  { path: '/tags', label: '태그' },
  { path: '/makers', label: '제작사' },
]

export default function TopNav() {
  const { searchQuery, setSearchQuery } = useLibraryStore()
  const navigate = useNavigate()

  return (
    <header className="flex items-center gap-6 px-4 h-12 bg-card border-b border-border shrink-0">
      <span className="text-primary font-bold text-sm tracking-wide mr-2">
        JAV Archivist
      </span>

      <nav className="flex items-center gap-1 flex-1">
        {TABS.map(({ path, label }) => (
          <NavLink
            key={path}
            to={path}
            className={({ isActive }) =>
              `px-3 py-1.5 text-sm rounded transition-colors ${
                isActive
                  ? 'text-primary border-b-2 border-primary pb-[5px]'
                  : 'text-muted-foreground hover:text-foreground'
              }`
            }
          >
            {label}
          </NavLink>
        ))}
      </nav>

      <div className="relative w-52">
        <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground" />
        <Input
          className="pl-7 h-7 text-xs bg-secondary border-border"
          placeholder="검색..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') navigate('/library')
          }}
        />
      </div>

      <NavLink
        to="/settings"
        className="text-muted-foreground hover:text-foreground transition-colors text-sm"
      >
        ⚙
      </NavLink>
    </header>
  )
}
