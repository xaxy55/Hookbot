import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { login } from '../api/client';

export default function LoginPage() {
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await login(password);
      navigate('/', { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{
      minHeight: '100vh',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#0a0a0f',
    }}>
      <form onSubmit={handleSubmit} style={{
        background: '#12121a',
        border: '1px solid #1e1e2e',
        borderRadius: '12px',
        padding: '2rem',
        width: '100%',
        maxWidth: '360px',
      }}>
        <h1 style={{
          fontSize: '1.5rem',
          fontWeight: 700,
          color: '#e0e0e0',
          marginBottom: '0.5rem',
          textAlign: 'center',
        }}>
          Hookbot
        </h1>
        <p style={{
          fontSize: '0.875rem',
          color: '#888',
          marginBottom: '1.5rem',
          textAlign: 'center',
        }}>
          Enter password to continue
        </p>

        {error && (
          <div style={{
            background: 'rgba(239, 68, 68, 0.1)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            borderRadius: '8px',
            padding: '0.75rem',
            marginBottom: '1rem',
            color: '#ef4444',
            fontSize: '0.875rem',
          }}>
            {error}
          </div>
        )}

        <input
          type="password"
          value={password}
          onChange={e => setPassword(e.target.value)}
          placeholder="Password"
          autoFocus
          style={{
            width: '100%',
            padding: '0.75rem',
            background: '#1a1a2e',
            border: '1px solid #2a2a3e',
            borderRadius: '8px',
            color: '#e0e0e0',
            fontSize: '1rem',
            outline: 'none',
            marginBottom: '1rem',
            boxSizing: 'border-box',
          }}
        />

        <button
          type="submit"
          disabled={loading || !password}
          style={{
            width: '100%',
            padding: '0.75rem',
            background: loading || !password ? '#333' : '#6366f1',
            color: '#fff',
            border: 'none',
            borderRadius: '8px',
            fontSize: '1rem',
            fontWeight: 600,
            cursor: loading || !password ? 'not-allowed' : 'pointer',
          }}
        >
          {loading ? 'Signing in...' : 'Sign in'}
        </button>
      </form>
    </div>
  );
}
