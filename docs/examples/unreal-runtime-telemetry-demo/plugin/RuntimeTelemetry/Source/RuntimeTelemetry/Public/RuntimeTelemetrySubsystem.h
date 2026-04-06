#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "RuntimeTelemetrySubsystem.generated.h"

USTRUCT(BlueprintType)
struct FRuntimeTelemetryEvent
{
    GENERATED_BODY()

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Telemetry")
    FString Name;

    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Telemetry")
    FString Payload;
};

UCLASS()
class RUNTIMETELEMETRY_API URuntimeTelemetrySubsystem : public UGameInstanceSubsystem
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category = "Telemetry")
    void RecordEvent(const FString& EventName, const FString& Payload);

    UFUNCTION(BlueprintCallable, Category = "Telemetry")
    void FlushEvents();

    UFUNCTION(BlueprintPure, Category = "Telemetry")
    int32 GetBufferedEventCount() const;

private:
    UPROPERTY(VisibleAnywhere, Category = "Telemetry")
    TArray<FRuntimeTelemetryEvent> BufferedEvents;
};
