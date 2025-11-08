/**
 * AI Service API
 * API endpoints for AI-related functionality
 */

import { apiClient, APIResponse } from '../api-client';

export interface AIStatus {
  isActive: boolean;
  version: string;
  llmEnabled: boolean;
  analyticsEnabled: boolean;
  monitoringEnabled: boolean;
  smartContractsEnabled: boolean;
}

export interface AIToggleRequest {
  enabled: boolean;
}

export interface AIToggleResponse {
  success: boolean;
  enabled: boolean;
  message: string;
}

export interface AIPredictionRequest {
  model: string;
  features: number[];
}

export interface AIPredictionResponse {
  prediction: number | number[];
  confidence: number;
  model: string;
  timestamp: number;
}

export interface AITrainingRequest {
  model: string;
  data: any[];
  config?: {
    epochs?: number;
    batchSize?: number;
    learningRate?: number;
  };
}

export interface AITrainingResponse {
  success: boolean;
  modelId: string;
  metrics: {
    accuracy?: number;
    loss?: number;
    [key: string]: any;
  };
}

/**
 * Get AI service status
 */
export async function getAIStatus(): Promise<APIResponse<AIStatus>> {
  return apiClient.get<AIStatus>('/api/ai/status');
}

/**
 * Toggle AI service on/off
 */
export async function toggleAI(enabled: boolean): Promise<APIResponse<AIToggleResponse>> {
  return apiClient.post<AIToggleResponse>('/api/ai/toggle', { enabled });
}

/**
 * Make an AI prediction
 */
export async function predict(
  request: AIPredictionRequest
): Promise<APIResponse<AIPredictionResponse>> {
  return apiClient.post<AIPredictionResponse>('/api/ai/predict', request);
}

/**
 * Train an AI model
 */
export async function trainModel(
  request: AITrainingRequest
): Promise<APIResponse<AITrainingResponse>> {
  return apiClient.post<AITrainingResponse>('/api/ai/train', request);
}

/**
 * Get available AI models
 */
export async function getModels(): Promise<APIResponse<string[]>> {
  return apiClient.get<string[]>('/api/ai/models');
}

/**
 * Get model information
 */
export async function getModelInfo(modelId: string): Promise<APIResponse<any>> {
  return apiClient.get(`/api/ai/models/${modelId}`);
}
