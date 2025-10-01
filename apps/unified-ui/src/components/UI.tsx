type CardProps = React.HTMLAttributes<HTMLDivElement> & {
  title?: React.ReactNode;
  children: React.ReactNode;
  footer?: React.ReactNode;
};

export function Card({ title, children, footer, className, ...rest }: CardProps) {
  return (
    <div
      {...rest}
      className={`border rounded-lg p-4 space-y-3 bg-white shadow-sm ${className || ''}`.trim()}
    >
      {title ? <h2 className="text-lg font-semibold text-gray-900">{title}</h2> : null}
      <div>{children}</div>
      {footer ? <div className="pt-2 border-t">{footer}</div> : null}
    </div>
  );
}

type ButtonVariant = 'primary' | 'secondary' | 'outline' | 'ghost' | 'default';
type ButtonSize = 'sm' | 'md' | 'lg';

export function Button({
  variant = 'primary',
  size = 'md',
  className,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant; size?: ButtonSize }) {
  const variants: Record<ButtonVariant, string> = {
    primary: 'bg-blue-600 text-white hover:bg-blue-700 focus-visible:ring-blue-500',
    secondary: 'bg-gray-200 text-gray-900 hover:bg-gray-300 focus-visible:ring-gray-400',
    outline: 'border border-gray-300 text-gray-900 hover:bg-gray-100 focus-visible:ring-gray-400 bg-transparent',
    ghost: 'text-gray-700 hover:bg-gray-100 focus-visible:ring-gray-400 bg-transparent',
    default: 'bg-gray-100 text-gray-900 hover:bg-gray-200 focus-visible:ring-gray-400'
  };

  const sizes: Record<ButtonSize, string> = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-sm',
    lg: 'px-5 py-3 text-base'
  };

  return (
    <button
      {...props}
      className={`inline-flex items-center justify-center rounded font-medium transition-colors disabled:opacity-50 disabled:pointer-events-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 ${variants[variant]} ${sizes[size]} ${className || ''}`.trim()}
    />
  );
}

type FieldProps = React.LabelHTMLAttributes<HTMLLabelElement> & {
  label: React.ReactNode;
  children: React.ReactNode;
  required?: boolean;
  helperText?: React.ReactNode;
};

export function Field({ label, children, required, helperText, className, ...props }: FieldProps) {
  return (
    <label className={`grid gap-1 ${className || ''}`.trim()} {...props}>
      <span className="text-sm text-gray-600 dark:text-gray-300 font-medium">
        {label}
        {required ? <span className="text-red-500 ml-1" aria-hidden="true">*</span> : null}
      </span>
      {children}
      {helperText ? <span className="text-xs text-gray-500">{helperText}</span> : null}
    </label>
  );
}

export function Input(props:React.InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={`border rounded px-3 py-2 bg-transparent focus:ring-2 focus:ring-blue-500 focus:border-transparent ${props.className||""}`} />
}

export function Textarea(props:React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...props} className={`border rounded px-3 py-2 bg-transparent focus:ring-2 focus:ring-blue-500 focus:border-transparent ${props.className||""}`} />
}

type BadgeVariant = 'default' | 'success' | 'warning' | 'error' | 'blue' | 'purple' | 'secondary' | 'outline';

export function Badge({children, variant = 'default', className}: {children: React.ReactNode; variant?: BadgeVariant; className?: string}) {
  const variants: Record<BadgeVariant, string> = {
    default: 'bg-gray-100 text-gray-800',
    success: 'bg-green-100 text-green-800',
    warning: 'bg-yellow-100 text-yellow-800',
    error: 'bg-red-100 text-red-800',
    blue: 'bg-blue-100 text-blue-800',
    purple: 'bg-purple-100 text-purple-800',
    secondary: 'bg-gray-200 text-gray-900',
    outline: 'border border-gray-300 text-gray-900 bg-transparent'
  };

  return (
    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${variants[variant]} ${className || ''}`.trim()}>
      {children}
    </span>
  );
}

export function LoadingSpinner() {
  return (
    <div className="flex justify-center items-center">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
    </div>
  );
}

export function Checkbox(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input 
      type="checkbox" 
      {...props} 
      className={`w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 focus:ring-2 ${props.className || ""}`}
    />
  );
}

export function Switch({ checked, onCheckedChange, ...props }: { checked: boolean; onCheckedChange: (checked: boolean) => void; [key: string]: any }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onCheckedChange(!checked)}
      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
        checked ? 'bg-blue-600' : 'bg-gray-200'
      }`}
      {...props}
    >
      <span
        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
          checked ? 'translate-x-6' : 'translate-x-1'
        }`}
      />
    </button>
  )
}

type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement> & {
  value: string;
  onValueChange?: (value: string) => void;
  children: React.ReactNode;
};

export function Select({ value, onValueChange, children, onChange, className, ...props }: SelectProps) {
  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    onValueChange?.(event.target.value);
    onChange?.(event);
  };

  return (
    <select
      value={value}
      onChange={handleChange}
      className={`block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm ${className || ''}`.trim()}
      {...props}
    >
      {children}
    </select>
  )
}

export function SelectTrigger({ children, ...props }: { children: React.ReactNode; [key: string]: any }) {
  return (
    <div 
      {...props}
      className={`w-full border rounded px-3 py-2 bg-transparent focus:ring-2 focus:ring-blue-500 focus:border-transparent cursor-pointer text-left ${props.className || ""}`}
    >
      {children}
    </div>
  );
}

export function SelectValue({ placeholder }: { placeholder?: string }) {
  return <span className="text-gray-600 block">{placeholder}</span>;
}

export function SelectContent({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}

export function SelectItem({ value, children }: { value: string; children: React.ReactNode }) {
  return <option value={value}>{children}</option>;
}

export function Sheet({ open, onOpenChange, children }: { open: boolean; onOpenChange: (open: boolean) => void; children: React.ReactNode }) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" onClick={() => onOpenChange(false)}>
      <div className="bg-white rounded-lg w-full max-w-xl max-h-[90vh] overflow-y-auto" onClick={(e) => e.stopPropagation()}>
        {children}
      </div>
    </div>
  );
}

export function SheetContent({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={`p-6 ${className || ""}`}>{children}</div>;
}

export function SheetHeader({ children }: { children: React.ReactNode }) {
  return <div className="mb-4">{children}</div>;
}

export function SheetTitle({ children, className }: { children: React.ReactNode; className?: string }) {
  return <h3 className={`text-lg font-semibold ${className || ""}`}>{children}</h3>;
}

export function SheetFooter({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={`flex items-center justify-between gap-2 pt-4 border-t ${className || ""}`}>{children}</div>;
}

export function Label({ children, className, ...props }: React.LabelHTMLAttributes<HTMLLabelElement>) {
  return (
    <label className={`text-sm font-medium ${className || ''}`.trim()} {...props}>
      {children}
    </label>
  );
}

export function Modal({ isOpen, onClose, title, children }: { 
  isOpen: boolean; 
  onClose: () => void; 
  title: string; 
  children: React.ReactNode 
}) {
  if (!isOpen) return null;
  
  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-white rounded-lg w-full max-w-2xl max-h-[90vh] overflow-y-auto mx-4" onClick={(e) => e.stopPropagation()}>
        <div className="p-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-semibold text-gray-900">{title}</h2>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-gray-600 text-2xl"
            >
              ×
            </button>
          </div>
          {children}
        </div>
      </div>
    </div>
  );
}