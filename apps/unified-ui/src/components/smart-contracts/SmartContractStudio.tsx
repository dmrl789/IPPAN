'use client';

import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { Code, Play, Save, Download, Upload, Brain, Shield, Zap } from 'lucide-react';

export function SmartContractStudio() {
  const [code, setCode] = useState(`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleStorage {
    uint256 private storedData;
    
    function set(uint256 x) public {
        storedData = x;
    }
    
    function get() public view returns (uint256) {
        return storedData;
    }
}`);

  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [analysisResult, setAnalysisResult] = useState(null);

  const analyzeCode = async () => {
    setIsAnalyzing(true);
    // Simulate AI analysis
    setTimeout(() => {
      setAnalysisResult({
        securityScore: 0.85,
        gasEfficiency: 0.92,
        issues: [
          { type: 'warning', message: 'Consider adding access control to set function' },
          { type: 'info', message: 'Contract follows best practices' }
        ],
        suggestions: [
          'Add onlyOwner modifier to set function',
          'Consider using events for state changes',
          'Add input validation'
        ]
      });
      setIsAnalyzing(false);
    }, 2000);
  };

  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700"
      >
        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
              Smart Contract Studio
            </h2>
            <div className="flex items-center space-x-2">
              <button className="px-4 py-2 bg-ai-500 hover:bg-ai-600 text-white rounded-lg flex items-center space-x-2 transition-colors">
                <Brain className="w-4 h-4" />
                <span>AI Assistant</span>
              </button>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 p-6">
          {/* Code Editor */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                Contract Code
              </h3>
              <div className="flex items-center space-x-2">
                <button className="p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                  <Upload className="w-4 h-4" />
                </button>
                <button className="p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                  <Download className="w-4 h-4" />
                </button>
                <button className="p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                  <Save className="w-4 h-4" />
                </button>
              </div>
            </div>
            
            <div className="border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden">
              <textarea
                value={code}
                onChange={(e) => setCode(e.target.value)}
                className="w-full h-96 p-4 bg-gray-50 dark:bg-slate-900 text-gray-900 dark:text-white font-mono text-sm resize-none focus:outline-none"
                placeholder="Enter your smart contract code here..."
              />
            </div>

            <div className="flex items-center space-x-2">
              <button
                onClick={analyzeCode}
                disabled={isAnalyzing}
                className="px-4 py-2 bg-blue-500 hover:bg-blue-600 disabled:bg-gray-400 text-white rounded-lg flex items-center space-x-2 transition-colors"
              >
                <Play className="w-4 h-4" />
                <span>{isAnalyzing ? 'Analyzing...' : 'Analyze with AI'}</span>
              </button>
            </div>
          </div>

          {/* AI Analysis Results */}
          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
              AI Analysis
            </h3>
            
            {isAnalyzing ? (
              <div className="flex items-center justify-center h-64">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-ai-500"></div>
              </div>
            ) : analysisResult ? (
              <div className="space-y-4">
                {/* Security Score */}
                <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
                  <div className="flex items-center space-x-2 mb-2">
                    <Shield className="w-5 h-5 text-green-500" />
                    <span className="font-medium text-gray-900 dark:text-white">Security Score</span>
                  </div>
                  <div className="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
                    <div 
                      className="bg-green-500 h-2 rounded-full transition-all duration-500"
                      style={{ width: `${analysisResult.securityScore * 100}%` }}
                    />
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                    {Math.round(analysisResult.securityScore * 100)}% - Good
                  </p>
                </div>

                {/* Gas Efficiency */}
                <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
                  <div className="flex items-center space-x-2 mb-2">
                    <Zap className="w-5 h-5 text-yellow-500" />
                    <span className="font-medium text-gray-900 dark:text-white">Gas Efficiency</span>
                  </div>
                  <div className="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
                    <div 
                      className="bg-yellow-500 h-2 rounded-full transition-all duration-500"
                      style={{ width: `${analysisResult.gasEfficiency * 100}%` }}
                    />
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                    {Math.round(analysisResult.gasEfficiency * 100)}% - Excellent
                  </p>
                </div>

                {/* Issues */}
                <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
                  <h4 className="font-medium text-gray-900 dark:text-white mb-2">Issues Found</h4>
                  <div className="space-y-2">
                    {analysisResult.issues.map((issue, index) => (
                      <div key={index} className={`p-2 rounded text-sm ${
                        issue.type === 'warning' ? 'bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200' :
                        issue.type === 'error' ? 'bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200' :
                        'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200'
                      }`}>
                        {issue.message}
                      </div>
                    ))}
                  </div>
                </div>

                {/* Suggestions */}
                <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
                  <h4 className="font-medium text-gray-900 dark:text-white mb-2">AI Suggestions</h4>
                  <ul className="space-y-1">
                    {analysisResult.suggestions.map((suggestion, index) => (
                      <li key={index} className="text-sm text-gray-600 dark:text-gray-400">
                        • {suggestion}
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            ) : (
              <div className="flex items-center justify-center h-64 text-gray-500 dark:text-gray-400">
                Click "Analyze with AI" to get started
              </div>
            )}
          </div>
        </div>
      </motion.div>
    </div>
  );
}