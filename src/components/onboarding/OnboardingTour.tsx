import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import { ArrowLeft2, ArrowRight2 } from '../ui/icons';
import { useOnboarding } from '../../hooks/useOnboarding';
import { ONBOARDING_STEPS } from './steps';

/**
 * First-run tour: four steps in the shared Modal, shown once and tracked in
 * localStorage by `useOnboarding`.
 *
 * Centred rather than a coach mark pinned to each real control. Every element
 * worth pointing at — the drop target, the tab bar, the footer — lives in a
 * component this does not own, so anchoring would mean matching on their
 * markup and pointing at the wrong pixel the first time any of them is laid
 * out differently. Four sentences in the middle of the window survive that.
 */
export function OnboardingTour() {
  const open = useOnboarding((s) => s.open);
  const step = useOnboarding((s) => s.step);
  const goTo = useOnboarding((s) => s.goTo);
  const finish = useOnboarding((s) => s.finish);

  const index = Math.min(step, ONBOARDING_STEPS.length - 1);
  const current = ONBOARDING_STEPS[index];
  const isLast = index === ONBOARDING_STEPS.length - 1;

  return (
    <Modal
      open={open}
      // Dismissing is finishing, whichever way it happens, so the tour cannot
      // reappear for someone who has already waved it away.
      onClose={finish}
      title={current.title}
      width="sm"
      footer={
        <>
          <Button variant="ghost" size="sm" className="mr-auto" onClick={finish}>
            Skip
          </Button>
          {index > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => goTo(index - 1)}
              icon={<ArrowLeft2 size={14} color="currentColor" variant="Linear" />}
            >
              Back
            </Button>
          )}
          <Button
            variant="primary"
            size="sm"
            onClick={() => (isLast ? finish() : goTo(index + 1))}
            icon={
              isLast ? undefined : <ArrowRight2 size={14} color="currentColor" variant="Linear" />
            }
          >
            {isLast ? 'Done' : 'Next'}
          </Button>
        </>
      }
    >
      <div className="flex items-start gap-3 py-1">
        <span className="mt-0.5 shrink-0 text-capstone-sky">{current.icon}</span>
        <p className="text-[13px] leading-relaxed text-gray-300">{current.body}</p>
      </div>

      <div
        className="mt-5 flex items-center gap-1.5"
        role="progressbar"
        aria-valuenow={index + 1}
        aria-valuemin={1}
        aria-valuemax={ONBOARDING_STEPS.length}
        aria-label="Tutorial progress"
      >
        {ONBOARDING_STEPS.map((s, i) => (
          <span
            key={s.title}
            className={`h-1 rounded-full transition-all ${
              i === index ? 'w-5 bg-capstone-sky' : 'w-1.5 bg-gray-600'
            }`}
          />
        ))}
      </div>
    </Modal>
  );
}
