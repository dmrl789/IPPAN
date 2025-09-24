import React from 'react';
import { Pressable, Text, StyleSheet, ViewStyle } from 'react-native';

type ButtonVariant = 'primary' | 'secondary' | 'ghost';

type ButtonProps = {
  label: string;
  onPress: () => void;
  disabled?: boolean;
  variant?: ButtonVariant;
  style?: ViewStyle | ViewStyle[];
};

export function Button({ label, onPress, disabled = false, variant = 'primary', style }: ButtonProps) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={disabled ? undefined : onPress}
      style={({ pressed }) => [
        styles.base,
        variantStyles[variant],
        disabled ? styles.disabled : null,
        pressed && !disabled ? styles.pressed : null,
        style
      ]}
    >
      <Text style={[styles.label, variant === 'ghost' ? styles.ghostLabel : null]}>{label}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  base: {
    borderRadius: 12,
    paddingVertical: 12,
    paddingHorizontal: 18,
    alignItems: 'center',
    justifyContent: 'center'
  },
  pressed: {
    opacity: 0.85
  },
  disabled: {
    opacity: 0.5
  },
  label: {
    fontSize: 15,
    fontWeight: '600',
    color: '#0f172a'
  },
  ghostLabel: {
    color: '#f8fafc'
  }
});

const variantStyles: Record<ButtonVariant, ViewStyle> = {
  primary: {
    backgroundColor: '#38bdf8'
  },
  secondary: {
    backgroundColor: '#1d4ed8'
  },
  ghost: {
    backgroundColor: 'transparent',
    borderWidth: 1,
    borderColor: '#38bdf8'
  }
};
