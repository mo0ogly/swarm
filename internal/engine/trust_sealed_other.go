//go:build linux && !amd64

package engine

func trustSealedChildProbe(input trustSealedInput) (trustSealedResult, error) {
	return trustSealedResult{}, trustSealedFailure()
}
