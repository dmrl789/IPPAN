import React from 'react';
import { View, Text, StyleSheet, ViewStyle } from 'react-native';

type CardProps = {
  title?: string;
  subtitle?: string;
  action?: React.ReactNode;
  children: React.ReactNode;
  style?: ViewStyle | ViewStyle[];
};

export function Card({ title, subtitle, action, children, style }: CardProps) {
  return (
    <View style={[styles.card, style]}> 
      {(title || action || subtitle) && (
        <View style={styles.header}>
          <View style={styles.headerTextContainer}>
            {title ? <Text style={styles.title}>{title}</Text> : null}
            {subtitle ? <Text style={styles.subtitle}>{subtitle}</Text> : null}
          </View>
          {action ? <View>{action}</View> : null}
        </View>
      )}
      <View style={styles.body}>{children}</View>
    </View>
  );
}

const styles = StyleSheet.create({
  card: {
    backgroundColor: '#111c33',
    borderRadius: 18,
    paddingHorizontal: 18,
    paddingVertical: 20,
    marginBottom: 16,
    shadowColor: '#000',
    shadowOpacity: 0.25,
    shadowRadius: 12,
    shadowOffset: { width: 0, height: 4 },
    elevation: 4,
    borderWidth: 1,
    borderColor: '#1f2a44'
  },
  header: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    marginBottom: 16
  },
  headerTextContainer: {
    flex: 1,
    paddingRight: 12
  },
  title: {
    fontSize: 18,
    fontWeight: '700',
    color: '#f8fafc',
    marginBottom: 4
  },
  subtitle: {
    fontSize: 13,
    color: '#94a3b8'
  },
  body: {
    gap: 12
  }
});
