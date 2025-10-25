'use client';

import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';

interface AIStatus {
  isActive: boolean;
  version: string;
  llmEnabled: boolean;
  analyticsEnabled: boolean;
  monitoringEnabled: boolean;
  smartContractsEnabled: boolean;
}

interface AIContextType {
  aiStatus: AIStatus;
  toggleAI: () => void;
  updateAIStatus: (status: Partial<AIStatus>) => void;
  isLoading: boolean;
  error: string | null;
}

const AIContext = createContext<AIContextType | undefined>(undefined);

export function AIProvider({ children }: { children: ReactNode }) {
  const [aiStatus, setAiStatus] = useState<AIStatus>({
    isActive: false,
    version: '1.0.0',
    llmEnabled: false,
    analyticsEnabled: false,
    monitoringEnabled: false,
    smartContractsEnabled: false,
  });
  
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Initialize AI status
  useEffect(() => {
    const initializeAI = async () => {
      try {
        setIsLoading(true);
        setError(null);
        
        // Check if AI is enabled from environment
        const aiEnabled = process.env.NEXT_PUBLIC_AI_ENABLED === '1';
        
        if (aiEnabled) {
          // Try to connect to AI service
          const response = await fetch('/api/ai/status');
          if (response.ok) {
            const status = await response.json();
            setAiStatus(status);
          } else {
            // Fallback to default enabled state
            setAiStatus({
              isActive: true,
              version: '1.0.0',
              llmEnabled: true,
              analyticsEnabled: true,
              monitoringEnabled: true,
              smartContractsEnabled: true,
            });
          }
        } else {
          setAiStatus(prev => ({ ...prev, isActive: false }));
        }
      } catch (err) {
        console.error('Failed to initialize AI:', err);
        setError('Failed to connect to AI service');
        setAiStatus(prev => ({ ...prev, isActive: false }));
      } finally {
        setIsLoading(false);
      }
    };

    initializeAI();
  }, []);

  const toggleAI = async () => {
    try {
      setError(null);
      const newStatus = !aiStatus.isActive;
      
      const response = await fetch('/api/ai/toggle', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ enabled: newStatus }),
      });

      if (response.ok) {
        setAiStatus(prev => ({ ...prev, isActive: newStatus }));
      } else {
        throw new Error('Failed to toggle AI status');
      }
    } catch (err) {
      console.error('Failed to toggle AI:', err);
      setError('Failed to toggle AI status');
    }
  };

  const updateAIStatus = (updates: Partial<AIStatus>) => {
    setAiStatus(prev => ({ ...prev, ...updates }));
  };

  const value: AIContextType = {
    aiStatus,
    toggleAI,
    updateAIStatus,
    isLoading,
    error,
  };

  return (
    <AIContext.Provider value={value}>
      {children}
    </AIContext.Provider>
  );
}

export function useAI() {
  const context = useContext(AIContext);
  if (context === undefined) {
    throw new Error('useAI must be used within an AIProvider');
  }
  return context;
}