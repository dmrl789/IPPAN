'use client';

import React from 'react';
import { motion } from 'framer-motion';
import { Brain, Settings, User, Moon, Sun } from 'lucide-react';
import { useTheme } from '@/contexts/ThemeContext';
import { useAI } from '@/contexts/AIContext';

export function Header() {
  const { theme, setTheme, actualTheme } = useTheme();
  const { aiStatus } = useAI();

  return (
    <header className="bg-white dark:bg-slate-800 shadow-lg border-b border-gray-200 dark:border-gray-700">
      <div className="px-6 py-4">
        <div className="flex items-center justify-between">
          {/* Logo and Title */}
          <motion.div 
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            className="flex items-center space-x-3"
          >
            <div className="w-10 h-10 bg-gradient-to-r from-ai-500 to-purple-600 rounded-lg flex items-center justify-center">
              <Brain className="w-6 h-6 text-white" />
            </div>
            <div>
              <h1 className="text-xl font-bold text-gray-900 dark:text-white">
                IPPAN AI
              </h1>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Unified Blockchain Interface
              </p>
            </div>
          </motion.div>

          {/* AI Status Indicator */}
          <motion.div 
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            className="flex items-center space-x-4"
          >
            <div className="flex items-center space-x-2 px-3 py-2 rounded-lg bg-gray-100 dark:bg-slate-700">
              <div className={`w-2 h-2 rounded-full ${aiStatus.isActive ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                AI {aiStatus.isActive ? 'Active' : 'Inactive'}
              </span>
            </div>

            {/* Theme Toggle */}
            <button
              onClick={() => setTheme(actualTheme === 'dark' ? 'light' : 'dark')}
              className="p-2 rounded-lg bg-gray-100 dark:bg-slate-700 hover:bg-gray-200 dark:hover:bg-slate-600 transition-colors"
            >
              {actualTheme === 'dark' ? (
                <Sun className="w-5 h-5 text-yellow-500" />
              ) : (
                <Moon className="w-5 h-5 text-gray-600" />
              )}
            </button>

            {/* Settings */}
            <button className="p-2 rounded-lg bg-gray-100 dark:bg-slate-700 hover:bg-gray-200 dark:hover:bg-slate-600 transition-colors">
              <Settings className="w-5 h-5 text-gray-600 dark:text-gray-400" />
            </button>

            {/* User Profile */}
            <button className="p-2 rounded-lg bg-gray-100 dark:bg-slate-700 hover:bg-gray-200 dark:hover:bg-slate-600 transition-colors">
              <User className="w-5 h-5 text-gray-600 dark:text-gray-400" />
            </button>
          </motion.div>
        </div>
      </div>
    </header>
  );
}