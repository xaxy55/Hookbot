import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getUsers, createUser, updateUser, deleteUser, getDevices, assignDeviceToUser, unassignDeviceFromUser } from '../api/client';
import { useToast } from '../hooks/useToast';

const ROLES = ['admin', 'user', 'viewer'];

export default function UsersPage() {
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);

  const { data: users = [], isLoading, isError } = useQuery({ queryKey: ['users'], queryFn: getUsers });
  const { data: devices = [] } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  const [form, setForm] = useState({ username: '', display_name: '', password: '', role: 'user' });
  const [editForm, setEditForm] = useState({ display_name: '', role: '', password: '' });

  const createMut = useMutation({
    mutationFn: () => createUser(form),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setShowForm(false);
      setForm({ username: '', display_name: '', password: '', role: 'user' });
      toast('User created', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const updateMut = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { display_name?: string; role?: string; password?: string } }) => updateUser(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setEditingId(null);
      toast('User updated', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => deleteUser(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['users'] });
      toast('User deleted', 'success');
    },
  });

  const assignMut = useMutation({
    mutationFn: ({ userId, deviceId }: { userId: string; deviceId: string }) => assignDeviceToUser(userId, deviceId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['users'] }),
  });

  const unassignMut = useMutation({
    mutationFn: ({ userId, deviceId }: { userId: string; deviceId: string }) => unassignDeviceFromUser(userId, deviceId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['users'] }),
  });

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-fg">Users</h1>
          <p className="text-sm text-muted mt-1">Manage user accounts and device assignments</p>
        </div>
        <button onClick={() => setShowForm(s => !s)} className="px-4 py-2 bg-brand text-white rounded-lg text-sm font-medium hover:bg-brand/90">
          {showForm ? 'Cancel' : 'Add User'}
        </button>
      </div>

      {showForm && (
        <div className="bg-surface border border-edge rounded-xl p-5 mb-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <label className="block">
              <span className="text-xs text-subtle uppercase">Username</span>
              <input value={form.username} onChange={e => setForm(f => ({ ...f, username: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="johndoe" />
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Display Name</span>
              <input value={form.display_name} onChange={e => setForm(f => ({ ...f, display_name: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="John Doe" />
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Password</span>
              <input type="password" value={form.password} onChange={e => setForm(f => ({ ...f, password: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" />
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Role</span>
              <select value={form.role} onChange={e => setForm(f => ({ ...f, role: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
                {ROLES.map(r => <option key={r} value={r}>{r}</option>)}
              </select>
            </label>
          </div>
          <button onClick={() => createMut.mutate()} disabled={!form.username || !form.password || !form.display_name}
            className="px-4 py-2 bg-brand text-white rounded-lg text-sm font-medium hover:bg-brand/90 disabled:opacity-50">
            Create User
          </button>
        </div>
      )}

      {isError ? (
        <div className="text-center py-12 text-muted">
          <p>Could not load users</p>
          <p className="text-sm mt-1 text-dim">Check your connection and try refreshing.</p>
        </div>
      ) : isLoading ? (
        <p className="text-muted text-sm">Loading...</p>
      ) : users.length === 0 ? (
        <div className="text-center py-12 text-muted">
          <p className="text-lg">No users yet</p>
          <p className="text-sm mt-1">Add users to manage device access</p>
        </div>
      ) : (
        <div className="space-y-4">
          {users.map(user => (
            <div key={user.id} className="bg-surface border border-edge rounded-xl p-5">
              {editingId === user.id ? (
                <div className="space-y-3">
                  <div className="grid grid-cols-3 gap-3">
                    <input value={editForm.display_name} onChange={e => setEditForm(f => ({ ...f, display_name: e.target.value }))}
                      className="rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="Display name" />
                    <select value={editForm.role} onChange={e => setEditForm(f => ({ ...f, role: e.target.value }))}
                      className="rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
                      {ROLES.map(r => <option key={r} value={r}>{r}</option>)}
                    </select>
                    <input type="password" value={editForm.password} onChange={e => setEditForm(f => ({ ...f, password: e.target.value }))}
                      className="rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="New password (optional)" />
                  </div>
                  <div className="flex gap-2">
                    <button onClick={() => updateMut.mutate({
                      id: user.id,
                      data: {
                        display_name: editForm.display_name || undefined,
                        role: editForm.role || undefined,
                        password: editForm.password || undefined,
                      }
                    })} className="px-3 py-1.5 bg-brand text-white rounded text-xs font-medium">Save</button>
                    <button onClick={() => setEditingId(null)} className="px-3 py-1.5 bg-inset text-muted rounded text-xs font-medium">Cancel</button>
                  </div>
                </div>
              ) : (
                <div className="flex items-start justify-between">
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-fg">{user.display_name}</span>
                      <span className="text-xs text-muted">@{user.username}</span>
                      <span className={`px-2 py-0.5 rounded-full text-[10px] font-medium uppercase ${
                        user.role === 'admin' ? 'bg-purple-500/10 text-purple-400' :
                        user.role === 'viewer' ? 'bg-gray-500/10 text-gray-400' :
                        'bg-blue-500/10 text-blue-400'
                      }`}>{user.role}</span>
                    </div>
                    <div className="text-xs text-dim mt-1">
                      Created {new Date(user.created_at).toLocaleDateString()}
                      {user.last_login_at && ` | Last login: ${new Date(user.last_login_at).toLocaleDateString()}`}
                    </div>
                    {/* Device assignments */}
                    <div className="mt-3 flex flex-wrap gap-2 items-center">
                      <span className="text-xs text-subtle">Devices:</span>
                      {user.device_ids.map(did => {
                        const dev = devices.find(d => d.id === did);
                        return (
                          <span key={did} className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-brand/10 text-xs text-brand-fg">
                            {dev?.name || did.slice(0, 8)}
                            <button onClick={() => unassignMut.mutate({ userId: user.id, deviceId: did })}
                              className="text-brand-fg/50 hover:text-brand-fg ml-0.5">&times;</button>
                          </span>
                        );
                      })}
                      <select onChange={e => {
                        if (e.target.value) {
                          assignMut.mutate({ userId: user.id, deviceId: e.target.value });
                          e.target.value = '';
                        }
                      }} className="rounded bg-inset border border-edge px-2 py-0.5 text-xs text-muted">
                        <option value="">+ assign</option>
                        {devices.filter(d => !user.device_ids.includes(d.id)).map(d =>
                          <option key={d.id} value={d.id}>{d.name}</option>
                        )}
                      </select>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button onClick={() => { setEditingId(user.id); setEditForm({ display_name: user.display_name, role: user.role, password: '' }); }}
                      className="px-3 py-1 rounded text-xs font-medium bg-inset text-muted hover:text-fg">Edit</button>
                    <button onClick={() => deleteMut.mutate(user.id)}
                      className="px-3 py-1 rounded text-xs font-medium bg-red-500/10 text-red-400 hover:bg-red-500/20">Delete</button>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
