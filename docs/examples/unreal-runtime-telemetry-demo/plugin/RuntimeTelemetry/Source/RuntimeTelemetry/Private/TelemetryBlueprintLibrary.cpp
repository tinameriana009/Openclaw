#include "TelemetryBlueprintLibrary.h"

#include "Engine/GameInstance.h"
#include "Engine/World.h"
#include "RuntimeTelemetrySubsystem.h"

void UTelemetryBlueprintLibrary::RecordTelemetryEvent(UObject* WorldContextObject, const FString& EventName, const FString& Payload)
{
    if (WorldContextObject == nullptr)
    {
        return;
    }

    UWorld* World = WorldContextObject->GetWorld();
    if (World == nullptr)
    {
        return;
    }

    UGameInstance* GameInstance = World->GetGameInstance();
    if (GameInstance == nullptr)
    {
        return;
    }

    if (URuntimeTelemetrySubsystem* TelemetrySubsystem = GameInstance->GetSubsystem<URuntimeTelemetrySubsystem>())
    {
        TelemetrySubsystem->RecordEvent(EventName, Payload);
    }
}
