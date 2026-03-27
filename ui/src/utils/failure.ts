export type FailureKind = 'timeout' | 'unavailable' | 'failure';

export type FailureView = {
  kind: FailureKind;
  label: string;
  tone: 'info' | 'danger';
  detailClass: FailureKind;
};

export const classifyFailureMessage = (message: string): FailureKind => {
  const normalized = message.trim().toLowerCase();

  if (normalized.includes('timed out') || normalized.includes('timeout')) {
    return 'timeout';
  }

  if (
    normalized.includes('unavailable') ||
    normalized.includes('connection refused') ||
    normalized.includes('failed to reach')
  ) {
    return 'unavailable';
  }

  return 'failure';
};

export const getFailureView = (message: string): FailureView => {
  const kind = classifyFailureMessage(message);

  return {
    kind,
    label: kind === 'timeout' ? 'Timeout' : kind === 'unavailable' ? 'Unavailable' : 'Failure',
    tone: kind === 'timeout' ? 'info' : 'danger',
    detailClass: kind,
  };
};
