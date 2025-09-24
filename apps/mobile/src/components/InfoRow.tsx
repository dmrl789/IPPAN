import React from 'react';
import { View, Text, StyleSheet, ViewStyle } from 'react-native';

type InfoRowProps = {
  label: string;
  value: React.ReactNode;
  style?: ViewStyle | ViewStyle[];
  align?: 'center' | 'top';
};

export function InfoRow({ label, value, style, align = 'center' }: InfoRowProps) {
  const valueElement =
    typeof value === 'string' || typeof value === 'number' ? (
      <Text style={styles.valueText}>{value}</Text>
    ) : (
      value
    );

  return (
    <View style={[styles.row, align === 'top' ? styles.alignTop : null, style]}>
      <Text style={styles.label}>{label}</Text>
      <View style={styles.value}>{valueElement}</View>
    </View>
  );
}

const styles = StyleSheet.create({
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: 12
  },
  alignTop: {
    alignItems: 'flex-start'
  },
  label: {
    flex: 1,
    fontSize: 14,
    color: '#cbd5f5',
    fontWeight: '600'
  },
  value: {
    flex: 1.2,
    alignItems: 'flex-end'
  },
  valueText: {
    fontSize: 15,
    color: '#f1f5f9',
    textAlign: 'right'
  }
});
