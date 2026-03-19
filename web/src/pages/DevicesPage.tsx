import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getGroups,
  createGroup,
  updateGroup,
  deleteGroup,
  addGroupMember,
  removeGroupMember,
  sendGroupState,
  type DeviceGroup,
} from '../api/client';
import DeviceCard from '../components/DeviceCard';

const GROUP_COLORS = [
  '#6366f1', '#8b5cf6', '#ec4899', '#ef4444', '#f97316',
  '#eab308', '#22c55e', '#14b8a6', '#06b6d4', '#3b82f6',
];

const AVATAR_STATES = ['idle', 'thinking', 'waiting', 'success', 'taskcheck', 'error'] as const;

export default function DevicesPage() {
  const queryClient = useQueryClient();
  const { data: devices, isLoading: devicesLoading } = useQuery({
    queryKey: ['devices'],
    queryFn: getDevices,
    refetchInterval: 5000,
  });
  const { data: groups, isLoading: groupsLoading } = useQuery({
    queryKey: ['groups'],
    queryFn: getGroups,
  });

  const [filterGroupId, setFilterGroupId] = useState<string | null>(null);
  const [showCreateGroup, setShowCreateGroup] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  const [newGroupColor, setNewGroupColor] = useState(GROUP_COLORS[0]);
  const [editingGroup, setEditingGroup] = useState<DeviceGroup | null>(null);
  const [editName, setEditName] = useState('');
  const [editColor, setEditColor] = useState('');
  const [managingGroup, setManagingGroup] = useState<DeviceGroup | null>(null);
  const [groupActionsOpen, setGroupActionsOpen] = useState<string | null>(null);

  const createGroupMut = useMutation({
    mutationFn: (data: { name: string; color: string }) => createGroup(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      setShowCreateGroup(false);
      setNewGroupName('');
      setNewGroupColor(GROUP_COLORS[0]);
    },
  });

  const updateGroupMut = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { name?: string; color?: string } }) =>
      updateGroup(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      setEditingGroup(null);
    },
  });

  const deleteGroupMut = useMutation({
    mutationFn: (id: string) => deleteGroup(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      if (filterGroupId) setFilterGroupId(null);
    },
  });

  const addMemberMut = useMutation({
    mutationFn: ({ groupId, deviceId }: { groupId: string; deviceId: string }) =>
      addGroupMember(groupId, deviceId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['groups'] }),
  });

  const removeMemberMut = useMutation({
    mutationFn: ({ groupId, deviceId }: { groupId: string; deviceId: string }) =>
      removeGroupMember(groupId, deviceId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['groups'] }),
  });

  const sendStateMut = useMutation({
    mutationFn: ({ groupId, state }: { groupId: string; state: string }) =>
      sendGroupState(groupId, state),
  });

  const filteredDevices = filterGroupId
    ? devices?.filter(d => {
        const group = groups?.find(g => g.id === filterGroupId);
        return group?.device_ids.includes(d.id);
      })
    : devices;

  const isLoading = devicesLoading || groupsLoading;

  return (
    <div>
      <h1 className="text-xl font-bold text-fg mb-6">Devices</h1>

      {/* Groups Section */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-semibold text-subtle uppercase tracking-wide">Groups</h2>
          <button
            onClick={() => setShowCreateGroup(!showCreateGroup)}
            className="text-xs px-3 py-1.5 rounded-md bg-indigo-600 text-white hover:bg-indigo-700 transition-colors"
          >
            + Create Group
          </button>
        </div>

        {/* Create Group Form */}
        {showCreateGroup && (
          <div className="mb-4 p-4 rounded-lg border border-edge bg-surface">
            <div className="flex items-end gap-3">
              <div className="flex-1">
                <label className="block text-xs text-subtle mb-1">Group Name</label>
                <input
                  type="text"
                  value={newGroupName}
                  onChange={e => setNewGroupName(e.target.value)}
                  placeholder="e.g. Living Room, Office..."
                  className="w-full px-3 py-1.5 rounded-md border border-edge bg-inset text-fg text-sm placeholder:text-dim focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Color</label>
                <div className="flex gap-1">
                  {GROUP_COLORS.map(c => (
                    <button
                      key={c}
                      onClick={() => setNewGroupColor(c)}
                      className={`w-6 h-6 rounded-full border-2 transition-all ${
                        newGroupColor === c ? 'border-fg scale-110' : 'border-transparent'
                      }`}
                      style={{ backgroundColor: c }}
                    />
                  ))}
                </div>
              </div>
              <button
                onClick={() => {
                  if (newGroupName.trim()) {
                    createGroupMut.mutate({ name: newGroupName.trim(), color: newGroupColor });
                  }
                }}
                disabled={!newGroupName.trim() || createGroupMut.isPending}
                className="px-4 py-1.5 rounded-md bg-indigo-600 text-white text-sm hover:bg-indigo-700 disabled:opacity-50 transition-colors"
              >
                {createGroupMut.isPending ? 'Creating...' : 'Create'}
              </button>
              <button
                onClick={() => setShowCreateGroup(false)}
                className="px-3 py-1.5 rounded-md border border-edge text-subtle text-sm hover:bg-raised transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {/* Edit Group Modal */}
        {editingGroup && (
          <div className="mb-4 p-4 rounded-lg border border-edge bg-surface">
            <h3 className="text-sm font-medium text-fg mb-3">Edit Group</h3>
            <div className="flex items-end gap-3">
              <div className="flex-1">
                <label className="block text-xs text-subtle mb-1">Name</label>
                <input
                  type="text"
                  value={editName}
                  onChange={e => setEditName(e.target.value)}
                  className="w-full px-3 py-1.5 rounded-md border border-edge bg-inset text-fg text-sm focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Color</label>
                <div className="flex gap-1">
                  {GROUP_COLORS.map(c => (
                    <button
                      key={c}
                      onClick={() => setEditColor(c)}
                      className={`w-6 h-6 rounded-full border-2 transition-all ${
                        editColor === c ? 'border-fg scale-110' : 'border-transparent'
                      }`}
                      style={{ backgroundColor: c }}
                    />
                  ))}
                </div>
              </div>
              <button
                onClick={() => {
                  if (editName.trim()) {
                    updateGroupMut.mutate({
                      id: editingGroup.id,
                      data: { name: editName.trim(), color: editColor },
                    });
                  }
                }}
                disabled={!editName.trim() || updateGroupMut.isPending}
                className="px-4 py-1.5 rounded-md bg-indigo-600 text-white text-sm hover:bg-indigo-700 disabled:opacity-50 transition-colors"
              >
                Save
              </button>
              <button
                onClick={() => setEditingGroup(null)}
                className="px-3 py-1.5 rounded-md border border-edge text-subtle text-sm hover:bg-raised transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {/* Group Pills */}
        {groups && groups.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-4">
            <button
              onClick={() => setFilterGroupId(null)}
              className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
                filterGroupId === null
                  ? 'border-indigo-500 bg-indigo-500/10 text-indigo-400'
                  : 'border-edge bg-surface text-subtle hover:bg-raised'
              }`}
            >
              All Devices
            </button>
            {groups.map(group => (
              <div key={group.id} className="relative">
                <button
                  onClick={() => setFilterGroupId(filterGroupId === group.id ? null : group.id)}
                  className={`text-xs px-3 py-1.5 rounded-full border transition-colors flex items-center gap-1.5 ${
                    filterGroupId === group.id
                      ? 'bg-opacity-20'
                      : 'bg-surface hover:bg-raised'
                  }`}
                  style={{
                    borderColor: filterGroupId === group.id ? group.color : undefined,
                    backgroundColor: filterGroupId === group.id ? `${group.color}15` : undefined,
                    color: filterGroupId === group.id ? group.color : undefined,
                  }}
                >
                  <span
                    className="w-2 h-2 rounded-full"
                    style={{ backgroundColor: group.color }}
                  />
                  {group.name}
                  <span className="text-dim ml-0.5">({group.device_ids.length})</span>
                </button>

                {/* Quick actions toggle */}
                <button
                  onClick={e => {
                    e.stopPropagation();
                    setGroupActionsOpen(groupActionsOpen === group.id ? null : group.id);
                  }}
                  className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-surface border border-edge text-dim text-[10px] flex items-center justify-center hover:bg-raised opacity-0 group-hover:opacity-100 hover:opacity-100 transition-opacity"
                  style={{ opacity: groupActionsOpen === group.id ? 1 : undefined }}
                >
                  ...
                </button>

                {/* Actions dropdown */}
                {groupActionsOpen === group.id && (
                  <div className="absolute top-8 left-0 z-20 min-w-[180px] rounded-lg border border-edge bg-surface shadow-lg py-1">
                    {/* Send State */}
                    <div className="px-3 py-1.5 text-[10px] uppercase text-dim tracking-wider">
                      Send State
                    </div>
                    <div className="flex flex-wrap gap-1 px-3 pb-2">
                      {AVATAR_STATES.map(s => (
                        <button
                          key={s}
                          onClick={() => {
                            sendStateMut.mutate({ groupId: group.id, state: s });
                            setGroupActionsOpen(null);
                          }}
                          className="text-[10px] px-2 py-0.5 rounded bg-inset text-subtle hover:bg-raised hover:text-fg transition-colors border border-edge"
                        >
                          {s}
                        </button>
                      ))}
                    </div>
                    <div className="border-t border-edge" />
                    <button
                      onClick={() => {
                        setManagingGroup(group);
                        setGroupActionsOpen(null);
                      }}
                      className="w-full text-left px-3 py-1.5 text-xs text-subtle hover:bg-raised hover:text-fg transition-colors"
                    >
                      Manage Members
                    </button>
                    <button
                      onClick={() => {
                        setEditingGroup(group);
                        setEditName(group.name);
                        setEditColor(group.color);
                        setGroupActionsOpen(null);
                      }}
                      className="w-full text-left px-3 py-1.5 text-xs text-subtle hover:bg-raised hover:text-fg transition-colors"
                    >
                      Edit Group
                    </button>
                    <button
                      onClick={() => {
                        if (confirm(`Delete group "${group.name}"?`)) {
                          deleteGroupMut.mutate(group.id);
                        }
                        setGroupActionsOpen(null);
                      }}
                      className="w-full text-left px-3 py-1.5 text-xs text-red-400 hover:bg-red-500/10 transition-colors"
                    >
                      Delete Group
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Manage Members Panel */}
        {managingGroup && (
          <div className="mb-4 p-4 rounded-lg border border-edge bg-surface">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-medium text-fg flex items-center gap-2">
                <span
                  className="w-3 h-3 rounded-full"
                  style={{ backgroundColor: managingGroup.color }}
                />
                Manage Members: {managingGroup.name}
              </h3>
              <button
                onClick={() => setManagingGroup(null)}
                className="text-xs text-subtle hover:text-fg transition-colors"
              >
                Close
              </button>
            </div>
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
              {devices?.map(d => {
                const isMember = managingGroup.device_ids.includes(d.id);
                return (
                  <button
                    key={d.id}
                    onClick={() => {
                      if (isMember) {
                        removeMemberMut.mutate({ groupId: managingGroup.id, deviceId: d.id });
                        // Optimistically update local state
                        setManagingGroup({
                          ...managingGroup,
                          device_ids: managingGroup.device_ids.filter(id => id !== d.id),
                        });
                      } else {
                        addMemberMut.mutate({ groupId: managingGroup.id, deviceId: d.id });
                        setManagingGroup({
                          ...managingGroup,
                          device_ids: [...managingGroup.device_ids, d.id],
                        });
                      }
                    }}
                    className={`flex items-center gap-2 px-3 py-2 rounded-md border text-xs text-left transition-colors ${
                      isMember
                        ? 'border-indigo-500/50 bg-indigo-500/10 text-fg'
                        : 'border-edge bg-inset text-subtle hover:bg-raised'
                    }`}
                  >
                    <span
                      className={`w-4 h-4 rounded border flex items-center justify-center text-[10px] ${
                        isMember
                          ? 'border-indigo-500 bg-indigo-500 text-white'
                          : 'border-edge'
                      }`}
                    >
                      {isMember ? '\u2713' : ''}
                    </span>
                    <div>
                      <span className="font-medium">{d.name}</span>
                      <span className={`ml-1.5 w-1.5 h-1.5 rounded-full inline-block ${d.online ? 'bg-green-500' : 'bg-dim'}`} />
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {/* Click-away handler for dropdowns */}
      {groupActionsOpen && (
        <div
          className="fixed inset-0 z-10"
          onClick={() => setGroupActionsOpen(null)}
        />
      )}

      {/* Device Grid */}
      {isLoading ? (
        <p className="text-subtle text-sm">Loading devices...</p>
      ) : filteredDevices && filteredDevices.length > 0 ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredDevices.map((d) => (
            <DeviceCard key={d.id} device={d} groups={groups} />
          ))}
        </div>
      ) : filterGroupId ? (
        <div className="text-center py-12">
          <p className="text-subtle mb-2">No devices in this group</p>
          <p className="text-dim text-sm">
            Click the group pill actions to add devices to this group.
          </p>
        </div>
      ) : (
        <div className="text-center py-12">
          <p className="text-subtle mb-2">No devices registered</p>
          <p className="text-dim text-sm">
            Go to Discovery to scan your network and register devices.
          </p>
        </div>
      )}
    </div>
  );
}
