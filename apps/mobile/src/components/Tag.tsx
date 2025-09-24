import React from 'react';
import { View, Text, StyleSheet, ViewStyle } from 'react-native';

type TagProps = {
  label: string;
  tone?: 'info' | 'success' | 'warning' | 'danger' | 'neutral';
  style?: ViewStyle | ViewStyle[];
};

const toneStyles: Record<NonNullable<TagProps['tone']>, { backgroundColor: string; color: string }> = {
  info: { backgroundColor: 'rgba(59, 130, 246, 0.15)', color: '#93c5fd' },
  success: { backgroundColor: 'rgba(16, 185, 129, 0.15)', color: '#6ee7b7' },
  warning: { backgroundColor: 'rgba(234, 179, 8, 0.15)', color: '#fde68a' },
  danger: { backgroundColor: 'rgba(248, 113, 113, 0.15)', color: '#fca5a5' },
  neutral: { backgroundColor: 'rgba(148, 163, 184, 0.15)', color: '#cbd5f5' }
};

export function Tag({ label, tone = 'neutral', style }: TagProps) {
  const palette = toneStyles[tone];
  return (
    <View style={[styles.container, { backgroundColor: palette.backgroundColor }, style]}>
      <Text style={[styles.text, { color: palette.color }]}>{label}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 999,
    alignSelf: 'flex-start'
  },
  text: {
    fontSize: 12,
    fontWeight: '600'
  }
});
