export interface Notification {
  id: number;
  title: string;
  message: string;
  details?: string;
  time: string;
  read: boolean;
  expanded: boolean;
}
