from rest_framework import serializers
from members.models import MemberRelationship


class RelationshipCreateSerializer(serializers.ModelSerializer):
    class Meta:
        model = MemberRelationship
        fields = ['from_member', 'to_member', 'relation_type']

    def validate(self, data):
        if data['from_member'] == data['to_member']:
            raise serializers.ValidationError("Source and Target members cannot be the same.")
        return data
