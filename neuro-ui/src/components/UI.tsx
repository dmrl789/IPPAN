export function Card(props:{title:string; children:React.ReactNode; footer?:React.ReactNode}) {
  return (
    <div className="border rounded-lg p-4 space-y-3">
      <h2 className="text-lg font-semibold">{props.title}</h2>
      <div>{props.children}</div>
      {props.footer && <div className="pt-2 border-t">{props.footer}</div>}
    </div>
  );
}
export function Button(props:React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button {...props} className={`px-4 py-2 rounded bg-blue-600 text-white disabled:opacity-50 ${props.className||""}`} />;
}
export function Field({label, children}:{label:string; children:React.ReactNode}) {
  return (
    <label className="grid gap-1">
      <span className="text-sm text-gray-600 dark:text-gray-300">{label}</span>
      {children}
    </label>
  );
}
export function Input(props:React.InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={`border rounded px-3 py-2 bg-transparent ${props.className||""}`} />
}
export function Textarea(props:React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...props} className={`border rounded px-3 py-2 bg-transparent ${props.className||""}`} />
}
